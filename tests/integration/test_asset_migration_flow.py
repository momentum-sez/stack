"""
Integration Test: Asset Migration Flow

Tests the complete asset migration workflow.
"""

import pytest
from datetime import datetime, timezone
from decimal import Decimal

from tools.phoenix.tensor import (
    ComplianceTensorV2,
    ComplianceState,
    ComplianceDomain,
)
from tools.phoenix.migration import (
    MigrationRequest,
    MigrationSaga,
    MigrationState,
)
from tools.phoenix.watcher import WatcherRegistry
from tools.phoenix.manifold import ComplianceManifold


class TestAssetMigrationFlow:
    """End-to-end asset migration tests."""

    def test_full_migration_success(self):
        """Test successful end-to-end migration."""
        request = MigrationRequest(
            asset_id="asset-001",
            asset_genesis_digest="sha256:genesis",
            source_jurisdiction="jurisdiction-a",
            target_jurisdiction="jurisdiction-b",
            requestor_did="did:owner:alice",
            asset_value_usd=Decimal("1000"),
        )

        saga = MigrationSaga(request)
        assert saga.state == MigrationState.INITIATED

        # Advance through all states
        for expected_state in [
            MigrationState.ATTESTATION_GATHERING,
            MigrationState.SOURCE_LOCK,
            MigrationState.TRANSIT,
            MigrationState.DESTINATION_VERIFICATION,
            MigrationState.COMPLETED,
        ]:
            saga.advance_state()
            assert saga.state == expected_state

        assert saga.evidence.final_state == MigrationState.COMPLETED

    def test_migration_cancellation(self):
        """Test migration cancellation."""
        request = MigrationRequest(
            asset_id="asset-001",
            asset_genesis_digest="sha256:genesis",
            source_jurisdiction="jurisdiction-a",
            target_jurisdiction="jurisdiction-b",
            requestor_did="did:owner:alice",
        )

        saga = MigrationSaga(request)
        saga.advance_state()

        result = saga.cancel("user_requested", "did:owner:alice")
        assert result is True
        assert saga.state == MigrationState.CANCELLED

    def test_migration_failure_compensation(self):
        """Test migration failure triggers compensation."""
        request = MigrationRequest(
            asset_id="asset-001",
            asset_genesis_digest="sha256:genesis",
            source_jurisdiction="jurisdiction-a",
            target_jurisdiction="jurisdiction-b",
        )

        saga = MigrationSaga(request)
        saga.advance_state()  # ATTESTATION_GATHERING
        saga.advance_state()  # SOURCE_LOCK
        saga.advance_state()  # TRANSIT

        saga.fail("timeout", "Transit phase timeout")
        assert saga.state == MigrationState.FAILED


class TestTensorIntegration:
    """Integration tests for tensor operations."""

    def test_tensor_compliance_query(self):
        """Test querying compliance state from tensor."""
        tensor = ComplianceTensorV2()

        # Use the correct API signature
        coord = tensor.set(
            asset_id="asset-001",
            jurisdiction_id="us",
            domain=ComplianceDomain.KYC,
            state=ComplianceState.COMPLIANT,
            reason_code="kyc_verified",
        )

        cell = tensor.get(coord)
        assert cell is not None
        assert cell.state == ComplianceState.COMPLIANT

    def test_tensor_cell_state_changes(self):
        """Test tensor cell state changes are tracked."""
        tensor = ComplianceTensorV2()

        # Set initial state
        coord = tensor.set(
            asset_id="asset-001",
            jurisdiction_id="us",
            domain=ComplianceDomain.AML,
            state=ComplianceState.PENDING,
            reason_code="awaiting_verification",
        )

        # Update to compliant
        tensor.set(
            asset_id="asset-001",
            jurisdiction_id="us",
            domain=ComplianceDomain.AML,
            state=ComplianceState.COMPLIANT,
            reason_code="verified",
        )

        cell = tensor.get(coord)
        assert cell.state == ComplianceState.COMPLIANT


class TestManifoldIntegration:
    """Integration tests for compliance manifold."""

    def test_manifold_initialization(self):
        """Test manifold initializes correctly."""
        manifold = ComplianceManifold()
        assert manifold is not None

    def test_standard_manifold_creation(self):
        """Test creating a standard manifold."""
        from tools.phoenix.manifold import create_standard_manifold

        manifold = create_standard_manifold()
        assert manifold is not None
        assert len(manifold._jurisdictions) > 0


class TestWatcherIntegration:
    """Integration tests for watcher registry."""

    def test_watcher_registry_operations(self):
        """Test basic watcher registry operations."""
        registry = WatcherRegistry()
        assert len(registry._watchers) == 0
