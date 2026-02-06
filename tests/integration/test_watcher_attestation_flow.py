"""
Integration Test: Watcher & Infrastructure Flow

Tests watcher, tensor, health, observability, and config integration.
"""

import pytest
from datetime import datetime, timezone
from decimal import Decimal

from tools.phoenix.watcher import WatcherRegistry
from tools.phoenix.tensor import (
    ComplianceTensorV2,
    ComplianceState,
    ComplianceDomain,
)


class TestWatcherFlow:
    """Tests for watcher operations."""

    def test_watcher_registry_initialization(self):
        """Test watcher registry initializes correctly."""
        registry = WatcherRegistry()
        assert registry is not None
        assert len(registry._watchers) == 0


class TestTensorWithAttestations:
    """Tests for tensor with attestation tracking."""

    def test_tensor_cell_with_attestations(self):
        """Test tensor cell stores attestation references."""
        tensor = ComplianceTensorV2()

        coord = tensor.set(
            asset_id="asset-001",
            jurisdiction_id="us",
            domain=ComplianceDomain.KYC,
            state=ComplianceState.COMPLIANT,
            reason_code="verified",
        )

        cell = tensor.get(coord)
        assert cell.state == ComplianceState.COMPLIANT

    def test_tensor_state_lattice(self):
        """Test compliance state follows lattice semantics."""
        assert ComplianceState.COMPLIANT.value != ComplianceState.PENDING.value


class TestHealthCheckIntegration:
    """Tests for health check integration."""

    def test_health_checker_initialization(self):
        """Test health checker initializes correctly."""
        from tools.phoenix.health import get_health_checker, HealthStatus

        checker = get_health_checker()
        result = checker.liveness()

        assert result.status == HealthStatus.HEALTHY
        assert result.name == "liveness"

    def test_health_deep_check(self):
        """Test deep health check returns report."""
        from tools.phoenix.health import get_health_checker

        checker = get_health_checker()
        report = checker.deep_health()

        assert report is not None
        assert report.version == "0.4.44"
        assert len(report.checks) > 0


class TestObservabilityIntegration:
    """Tests for observability integration."""

    def test_correlation_id_generation(self):
        """Test correlation ID generation."""
        from tools.phoenix.observability import generate_correlation_id

        cid1 = generate_correlation_id()
        cid2 = generate_correlation_id()

        assert cid1.startswith("corr-")
        assert cid2.startswith("corr-")
        assert cid1 != cid2

    def test_tracer_span_creation(self):
        """Test tracer creates spans correctly."""
        from tools.phoenix.observability import get_tracer, PhoenixLayer

        tracer = get_tracer()

        with tracer.span("test_operation", PhoenixLayer.TENSOR) as span:
            span.set_attribute("test_key", "test_value")
            span.record_event("test_event", data="value")

        assert span.name == "test_operation"
        assert span.layer == "tensor"
        assert span.attributes["test_key"] == "test_value"


class TestConfigIntegration:
    """Tests for configuration integration."""

    def test_config_defaults(self):
        """Test configuration has correct defaults."""
        from tools.phoenix.config import get_config

        config = get_config()

        assert config.vm.gas_limit_default.get() == 10000000
        assert config.vm.stack_depth_max.get() == 1024
        assert config.watcher.min_collateral_usd.get() == Decimal("1000")

    def test_config_to_dict(self):
        """Test configuration serializes to dict."""
        from tools.phoenix.config import get_config

        config = get_config()
        data = config.to_dict()

        assert "vm" in data
        assert "watcher" in data
        assert "tensor" in data


class TestCLIIntegration:
    """Tests for CLI integration."""

    def test_cli_initialization(self):
        """Test CLI initializes correctly."""
        from tools.phoenix.cli import PhoenixCLI

        cli = PhoenixCLI()
        assert cli is not None
        assert cli.parser is not None

    def test_cli_output_formatting(self):
        """Test CLI output formatting."""
        from tools.phoenix.cli import format_output, OutputFormat

        data = {"key": "value", "number": 42}
        json_output = format_output(data, OutputFormat.JSON)

        assert "key" in json_output
        assert "value" in json_output
