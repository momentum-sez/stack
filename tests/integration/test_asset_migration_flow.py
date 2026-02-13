"""
Integration Test: Complete PHOENIX System Flow

Production-grade integration tests validating end-to-end workflows
across all PHOENIX components with the actual API.
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
from tools.phoenix.manifold import ComplianceManifold, create_standard_manifold


class TestMigrationSagaFlow:
    """Tests for migration saga state machine."""

    def test_saga_initialization(self):
        """Test saga initializes correctly."""
        request = MigrationRequest(
            asset_id="asset-001",
            asset_genesis_digest="sha256:genesis123",
            source_jurisdiction="jurisdiction-a",
            target_jurisdiction="jurisdiction-b",
        )

        saga = MigrationSaga(request)
        assert saga.state == MigrationState.INITIATED

    def test_saga_state_transitions(self):
        """Test saga transitions through valid states."""
        request = MigrationRequest(
            asset_id="asset-001",
            asset_genesis_digest="sha256:genesis123",
            source_jurisdiction="jurisdiction-a",
            target_jurisdiction="jurisdiction-b",
        )

        saga = MigrationSaga(request)

        # Check valid transitions — INITIATED → COMPLIANCE_CHECK first
        assert saga.can_transition_to(MigrationState.COMPLIANCE_CHECK)

        # Advance through COMPLIANCE_CHECK to ATTESTATION_GATHERING
        saga.advance_to(MigrationState.COMPLIANCE_CHECK)
        assert saga.state == MigrationState.COMPLIANCE_CHECK

        assert saga.can_transition_to(MigrationState.ATTESTATION_GATHERING)
        saga.advance_to(MigrationState.ATTESTATION_GATHERING)
        assert saga.state == MigrationState.ATTESTATION_GATHERING

    def test_saga_cancellation(self):
        """Test saga cancellation."""
        request = MigrationRequest(
            asset_id="asset-001",
            asset_genesis_digest="sha256:genesis123",
            source_jurisdiction="jurisdiction-a",
            target_jurisdiction="jurisdiction-b",
        )

        saga = MigrationSaga(request)
        saga.advance_to(MigrationState.COMPLIANCE_CHECK)
        saga.advance_to(MigrationState.ATTESTATION_GATHERING)

        result = saga.cancel("user_requested")
        assert result is True
        assert saga.state == MigrationState.CANCELLED

    def test_saga_serialization(self):
        """Test saga serializes to dict."""
        request = MigrationRequest(
            asset_id="asset-001",
            asset_genesis_digest="sha256:genesis123",
            source_jurisdiction="jurisdiction-a",
            target_jurisdiction="jurisdiction-b",
        )

        saga = MigrationSaga(request)
        data = saga.to_dict()

        assert "state" in data
        assert "request" in data


class TestTensorOperations:
    """Tests for compliance tensor operations."""

    def test_tensor_set_and_get(self):
        """Test setting and getting tensor cells."""
        tensor = ComplianceTensorV2()

        coord = tensor.set(
            asset_id="asset-001",
            jurisdiction_id="us",
            domain=ComplianceDomain.KYC,
            state=ComplianceState.COMPLIANT,
            reason_code="verified",
        )

        # Get using the returned coordinate
        cell = tensor.get(
            asset_id="asset-001",
            jurisdiction_id="us",
            domain=ComplianceDomain.KYC,
        )

        assert cell is not None
        assert cell.state == ComplianceState.COMPLIANT

    def test_tensor_state_update(self):
        """Test updating tensor cell state."""
        tensor = ComplianceTensorV2()

        # Set initial state
        tensor.set(
            asset_id="asset-001",
            jurisdiction_id="us",
            domain=ComplianceDomain.AML,
            state=ComplianceState.PENDING,
        )

        # Update state
        tensor.set(
            asset_id="asset-001",
            jurisdiction_id="us",
            domain=ComplianceDomain.AML,
            state=ComplianceState.COMPLIANT,
        )

        cell = tensor.get(
            asset_id="asset-001",
            jurisdiction_id="us",
            domain=ComplianceDomain.AML,
        )

        assert cell.state == ComplianceState.COMPLIANT

    def test_tensor_multiple_domains(self):
        """Test tensor with multiple compliance domains."""
        tensor = ComplianceTensorV2()

        domains = [ComplianceDomain.KYC, ComplianceDomain.AML, ComplianceDomain.SANCTIONS]

        for domain in domains:
            tensor.set(
                asset_id="asset-001",
                jurisdiction_id="us",
                domain=domain,
                state=ComplianceState.COMPLIANT,
            )

        for domain in domains:
            cell = tensor.get(
                asset_id="asset-001",
                jurisdiction_id="us",
                domain=domain,
            )
            assert cell.state == ComplianceState.COMPLIANT


class TestManifoldOperations:
    """Tests for compliance manifold operations."""

    def test_manifold_creation(self):
        """Test manifold creation."""
        manifold = ComplianceManifold()
        assert manifold is not None

    def test_standard_manifold(self):
        """Test standard manifold factory."""
        manifold = create_standard_manifold()
        assert manifold is not None
        assert len(manifold._jurisdictions) > 0

    def test_manifold_jurisdiction_lookup(self):
        """Test looking up jurisdictions in manifold."""
        manifold = create_standard_manifold()

        # Standard manifold should have some jurisdictions
        jurisdictions = list(manifold._jurisdictions.keys())
        assert len(jurisdictions) > 0


class TestWatcherOperations:
    """Tests for watcher registry operations."""

    def test_registry_initialization(self):
        """Test watcher registry initialization."""
        registry = WatcherRegistry()
        assert len(registry._watchers) == 0


class TestHealthInfrastructure:
    """Tests for health check infrastructure."""

    def test_health_checker_liveness(self):
        """Test liveness probe."""
        from tools.phoenix.health import get_health_checker, HealthStatus

        checker = get_health_checker()
        result = checker.liveness()

        assert result.status == HealthStatus.HEALTHY
        assert result.name == "liveness"
        assert "pid" in result.metadata

    def test_health_checker_readiness(self):
        """Test readiness probe."""
        from tools.phoenix.health import get_health_checker

        checker = get_health_checker()
        checker.mark_initialized()
        result = checker.readiness()

        assert result.name == "readiness"

    def test_health_deep_check(self):
        """Test deep health check."""
        from tools.phoenix.health import get_health_checker

        checker = get_health_checker()
        report = checker.deep_health()

        assert report.version == "0.4.44"
        assert len(report.checks) >= 3  # memory, threads, gc

    def test_metrics_collector(self):
        """Test metrics collector."""
        from tools.phoenix.health import get_metrics

        metrics = get_metrics()
        metrics.inc_counter("test_counter")
        metrics.set_gauge("test_gauge", 42.0)

        output = metrics.to_prometheus()
        assert "test_counter" in output
        assert "test_gauge" in output


class TestObservabilityInfrastructure:
    """Tests for observability infrastructure."""

    def test_correlation_id(self):
        """Test correlation ID generation."""
        from tools.phoenix.observability import generate_correlation_id, get_correlation_id

        cid = generate_correlation_id()
        assert cid.startswith("corr-")
        assert len(cid) == 17  # corr- + 12 hex chars

    def test_tracer_span(self):
        """Test tracer span creation."""
        from tools.phoenix.observability import get_tracer, PhoenixLayer

        tracer = get_tracer()

        with tracer.span("test_op", PhoenixLayer.TENSOR) as span:
            span.set_attribute("key", "value")
            span.record_event("event_name")

        assert span.name == "test_op"
        assert span.layer == "tensor"
        assert span.duration_ms >= 0

    def test_phoenix_logger(self):
        """Test structured logger."""
        from tools.phoenix.observability import PhoenixLogger, PhoenixLayer

        logger = PhoenixLogger("test", PhoenixLayer.VM)
        # Should not raise
        logger.info("test message", key="value")
        logger.debug("debug message")


class TestConfigInfrastructure:
    """Tests for configuration infrastructure."""

    def test_config_defaults(self):
        """Test configuration defaults."""
        from tools.phoenix.config import get_config

        config = get_config()

        assert config.vm.gas_limit_default.get() == 10000000
        assert config.vm.stack_depth_max.get() == 1024
        assert config.watcher.min_collateral_usd.get() == Decimal("1000")
        assert config.tensor.cache_ttl_seconds.get() == 300

    def test_config_serialization(self):
        """Test configuration serialization."""
        from tools.phoenix.config import get_config

        config = get_config()
        data = config.to_dict()

        assert "vm" in data
        assert "watcher" in data
        assert "tensor" in data
        assert "anchor" in data
        assert "migration" in data
        assert "security" in data

    def test_config_yaml_export(self):
        """Test YAML export."""
        from tools.phoenix.config import get_config

        config = get_config()
        yaml_str = config.to_yaml()

        assert "vm:" in yaml_str
        assert "gas_limit_default:" in yaml_str


class TestCLIInfrastructure:
    """Tests for CLI infrastructure."""

    def test_cli_creation(self):
        """Test CLI creation."""
        from tools.phoenix.cli import PhoenixCLI

        cli = PhoenixCLI()
        assert cli.parser is not None

    def test_cli_output_formats(self):
        """Test CLI output formatting."""
        from tools.phoenix.cli import format_output, OutputFormat

        data = {"key": "value", "number": 42}

        json_out = format_output(data, OutputFormat.JSON)
        assert '"key"' in json_out

        text_out = format_output(data, OutputFormat.TEXT)
        assert "key" in text_out

    def test_cli_health_command(self):
        """Test CLI health command handler."""
        from tools.phoenix.cli import PhoenixCLI
        import argparse

        cli = PhoenixCLI()
        args = argparse.Namespace(
            command="health",
            subcommand="version",
            format="json",
            quiet=False,
        )

        result = cli._handle_health_version(args)
        assert "version" in result
        assert result["version"] == "0.4.44"
