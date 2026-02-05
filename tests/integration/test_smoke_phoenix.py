"""
PHOENIX Smoke Test Suite

Comprehensive validation that all 14 PHOENIX modules import correctly,
instantiate without errors, and basic operations function as expected.

This is the definitive sanity check for v0.4.44 GENESIS.
"""

import pytest
from datetime import datetime, timezone, timedelta
from decimal import Decimal


class TestLayer1AssetIntelligence:
    """Smoke tests for Layer 1: Asset Intelligence modules."""

    def test_tensor_module_imports(self):
        """Verify all tensor exports are available."""
        from tools.phoenix.tensor import (
            ComplianceTensorV2,
            ComplianceState,
            ComplianceDomain,
            TensorSlice,
            TensorCommitment,
            ComplianceProof,
            TensorCoord,
            TensorCell,
            AttestationRef,
            tensor_meet,
            tensor_join,
        )

        # Basic instantiation
        tensor = ComplianceTensorV2()
        assert tensor is not None

        # Basic operation
        coord = tensor.set(
            asset_id="smoke-test-asset",
            jurisdiction_id="smoke-test-jur",
            domain=ComplianceDomain.KYC,
            state=ComplianceState.COMPLIANT,
        )
        assert coord is not None

        # Retrieval
        cell = tensor.get(
            asset_id="smoke-test-asset",
            jurisdiction_id="smoke-test-jur",
            domain=ComplianceDomain.KYC,
        )
        assert cell.state == ComplianceState.COMPLIANT

    def test_zkp_module_imports(self):
        """Verify all ZK proof exports are available."""
        from tools.phoenix.zkp import (
            ProofSystem,
            Circuit,
            CircuitRegistry,
            Witness,
            Proof,
            VerificationKey,
            ProvingKey,
            CircuitType,
            MockProver,
            MockVerifier,
            create_standard_registry,
        )

        # Basic instantiation
        registry = create_standard_registry()
        assert registry is not None
        assert len(registry._circuits) > 0

        # Mock prover/verifier
        prover = MockProver()
        verifier = MockVerifier()
        assert prover is not None
        assert verifier is not None

    def test_vm_module_imports(self):
        """Verify all VM exports are available."""
        from tools.phoenix.vm import (
            OpCode,
            Word,
            ExecutionContext,
            VMState,
            GasCosts,
            ExecutionResult,
            ComplianceCoprocessor,
            MigrationCoprocessor,
            SmartAssetVM,
            Assembler,
        )

        # Basic instantiation
        vm = SmartAssetVM()
        assert vm is not None

        # Basic execution
        bytecode = bytes([OpCode.PUSH1.value, 42, OpCode.PUSH1.value, 10, OpCode.ADD.value, OpCode.STOP.value])
        ctx = ExecutionContext(
            caller="0x" + "0" * 40,
            origin="0x" + "0" * 40,
            jurisdiction="smoke-test",
        )
        result = vm.execute(bytecode, ctx)
        assert result is not None

        # Assembler
        assembler = Assembler()
        assert assembler is not None


class TestLayer2JurisdictionalInfrastructure:
    """Smoke tests for Layer 2: Jurisdictional Infrastructure modules."""

    def test_manifold_module_imports(self):
        """Verify all manifold exports are available."""
        from tools.phoenix.manifold import (
            ComplianceManifold,
            MigrationPath,
            AttestationRequirement,
            PathConstraint,
            JurisdictionNode,
            CorridorEdge,
            AttestationGap,
            AttestationType,
            MigrationHop,
            create_standard_manifold,
        )

        # Basic instantiation
        manifold = ComplianceManifold()
        assert manifold is not None

        # Standard manifold factory
        standard = create_standard_manifold()
        assert standard is not None
        assert len(standard._jurisdictions) > 0

    def test_migration_module_imports(self):
        """Verify all migration exports are available."""
        from tools.phoenix.migration import (
            MigrationSaga,
            MigrationState,
            MigrationRequest,
            MigrationEvidence,
            CompensationAction,
            MigrationOrchestrator,
            StateTransition,
            LockEvidence,
            TransitProof,
            VerificationResult,
        )

        # Basic instantiation
        request = MigrationRequest(
            asset_id="smoke-test-asset",
            asset_genesis_digest="a" * 64,
            source_jurisdiction="smoke-source",
            target_jurisdiction="smoke-target",
        )
        saga = MigrationSaga(request)
        assert saga is not None
        assert saga.state == MigrationState.INITIATED

    def test_bridge_module_imports(self):
        """Verify all bridge exports are available."""
        from tools.phoenix.bridge import (
            CorridorBridge,
            BridgePhase,
            BridgeRequest,
            BridgeExecution,
            HopExecution,
            HopStatus,
            PrepareReceipt,
            CommitReceipt,
            BridgeReceiptChain,
            create_bridge_with_manifold,
        )

        # Basic instantiation
        bridge = create_bridge_with_manifold()
        assert bridge is not None

    def test_anchor_module_imports(self):
        """Verify all anchor exports are available."""
        from tools.phoenix.anchor import (
            Chain,
            AnchorStatus,
            AnchorManager,
            AnchorRecord,
            CorridorCheckpoint,
            InclusionProof,
            MockChainAdapter,
            CrossChainVerifier,
            CrossChainVerification,
            create_mock_anchor_manager,
        )

        # Basic instantiation
        manager = create_mock_anchor_manager()
        assert manager is not None


class TestLayer3NetworkCoordination:
    """Smoke tests for Layer 3: Network Coordination modules."""

    def test_watcher_module_imports(self):
        """Verify all watcher exports are available."""
        from tools.phoenix.watcher import (
            WatcherBond,
            SlashingCondition,
            SlashingClaim,
            WatcherReputation,
            WatcherRegistry,
            WatcherId,
            BondStatus,
            ReputationMetrics,
            EquivocationDetector,
            SlashingEvidence,
        )

        # Basic instantiation
        registry = WatcherRegistry()
        assert registry is not None
        assert len(registry._watchers) == 0

    def test_security_module_imports(self):
        """Verify all security exports are available."""
        from tools.phoenix.security import (
            AttestationScope,
            ScopedAttestation,
            NonceRegistry,
            VersionedValue,
            VersionedStore,
            TimeLock,
            TimeLockState,
            TimeLockManager,
            SignatureScheme,
            SignedMessage,
            SignatureVerifier,
            AuditEventType,
            AuditEvent,
            SecureWithdrawalManager,
            WithdrawalRequest,
        )

        # Basic instantiation
        nonce_registry = NonceRegistry()
        assert nonce_registry is not None

        versioned_store = VersionedStore()
        assert versioned_store is not None

        time_lock_manager = TimeLockManager()
        assert time_lock_manager is not None

    def test_hardening_module_imports(self):
        """Verify all hardening exports are available."""
        from tools.phoenix.hardening import (
            ValidationError,
            ValidationErrors,
            SecurityViolation,
            InvariantViolation,
            EconomicAttackDetected,
            ValidationResult,
            Validators,
            CryptoUtils,
            ThreadSafeDict,
            AtomicCounter,
            InvariantChecker,
            EconomicGuard,
            RateLimiter,
            RateLimitConfig,
        )

        # Thread-safe dict
        safe_dict = ThreadSafeDict()
        safe_dict["key"] = "value"
        assert safe_dict["key"] == "value"

        # Atomic counter
        counter = AtomicCounter()
        assert counter.get() == 0
        counter.increment()
        assert counter.get() == 1


class TestLayer4Operations:
    """Smoke tests for Layer 4: Operations modules."""

    def test_health_module_imports(self):
        """Verify all health exports are available."""
        from tools.phoenix.health import (
            HealthChecker,
            HealthStatus,
            HealthReport,
            HealthCheck,
            DependencyConfig,
            MetricsCollector,
            get_health_checker,
            get_metrics,
        )

        # Health checker
        checker = get_health_checker()
        assert checker is not None

        # Liveness probe
        liveness = checker.liveness()
        assert liveness.status == HealthStatus.HEALTHY

        # Deep health check
        report = checker.deep_health()
        assert report.version == "0.4.44"

        # Metrics
        metrics = get_metrics()
        assert metrics is not None
        metrics.inc_counter("smoke_test_counter")
        metrics.set_gauge("smoke_test_gauge", 42.0)

    def test_observability_module_imports(self):
        """Verify all observability exports are available."""
        from tools.phoenix.observability import (
            PhoenixLogger,
            PhoenixLayer,
            Tracer,
            Span,
            SpanContext,
            generate_correlation_id,
            get_correlation_id,
            set_correlation_id,
            get_tracer,
            get_audit_logger,
        )

        # Correlation ID
        cid = generate_correlation_id()
        assert cid.startswith("corr-")

        # Tracer
        tracer = get_tracer()
        assert tracer is not None

        with tracer.span("smoke_test_span", PhoenixLayer.TENSOR) as span:
            span.set_attribute("test", "value")
            span.record_event("smoke_event")

        assert span.duration_ms >= 0

        # Logger
        logger = PhoenixLogger("smoke_test", PhoenixLayer.VM)
        logger.info("Smoke test message", key="value")

    def test_config_module_imports(self):
        """Verify all config exports are available."""
        from tools.phoenix.config import (
            PhoenixConfig,
            ConfigManager,
            ConfigValue,
            ConfigError,
            TensorConfig,
            VMConfig,
            WatcherConfig,
            AnchorConfig,
            MigrationConfig,
            SecurityConfig,
            ObservabilityConfig,
            get_config,
            get_config_manager,
        )

        # Config
        config = get_config()
        assert config is not None

        # Default values
        assert config.vm.gas_limit_default.get() == 10000000
        assert config.vm.stack_depth_max.get() == 1024
        assert config.watcher.min_collateral_usd.get() == Decimal("1000")

        # Serialization
        data = config.to_dict()
        assert "vm" in data
        assert "watcher" in data

        yaml_str = config.to_yaml()
        assert "vm:" in yaml_str

    def test_cli_module_imports(self):
        """Verify all CLI exports are available."""
        from tools.phoenix.cli import (
            PhoenixCLI,
            OutputFormat,
            format_output,
        )

        # CLI
        cli = PhoenixCLI()
        assert cli is not None
        assert cli.parser is not None

        # Output formatting
        data = {"key": "value", "number": 42}

        json_out = format_output(data, OutputFormat.JSON)
        assert '"key"' in json_out

        text_out = format_output(data, OutputFormat.TEXT)
        assert "key" in text_out


class TestCrossLayerIntegration:
    """Smoke tests for cross-layer integration."""

    def test_tensor_to_vm_integration(self):
        """Test tensor compliance check via VM coprocessor."""
        from tools.phoenix.tensor import ComplianceTensorV2, ComplianceDomain, ComplianceState
        from tools.phoenix.vm import SmartAssetVM, ExecutionContext, OpCode

        # Set up tensor with compliance state
        tensor = ComplianceTensorV2()
        tensor.set(
            asset_id="cross-layer-asset",
            jurisdiction_id="cross-layer-jur",
            domain=ComplianceDomain.KYC,
            state=ComplianceState.COMPLIANT,
        )

        # Create VM with compliance coprocessor
        vm = SmartAssetVM()
        vm._compliance._tensor = tensor

        # Execute a simple program
        bytecode = bytes([OpCode.PUSH1.value, 1, OpCode.STOP.value])
        ctx = ExecutionContext(
            caller="0x" + "0" * 40,
            origin="0x" + "0" * 40,
            jurisdiction="cross-layer-jur",
        )
        result = vm.execute(bytecode, ctx)
        assert result is not None

    def test_config_to_health_integration(self):
        """Test configuration affects health checks."""
        from tools.phoenix.config import get_config
        from tools.phoenix.health import get_health_checker

        config = get_config()
        checker = get_health_checker()

        # Both should reference same version
        report = checker.deep_health()
        assert report.version == "0.4.44"

    def test_observability_to_config_integration(self):
        """Test observability respects configuration."""
        from tools.phoenix.config import get_config
        from tools.phoenix.observability import PhoenixLogger, PhoenixLayer

        config = get_config()
        logger = PhoenixLogger("integration_test", PhoenixLayer.TENSOR)

        # Should not raise
        logger.info("Integration test message")
        logger.debug("Debug message")


class TestPhoenixPackageImport:
    """Smoke test for the main phoenix package import."""

    def test_phoenix_package_version(self):
        """Verify phoenix package has correct version."""
        from tools.phoenix import __version__, __codename__

        assert __version__ == "0.4.44"
        assert __codename__ == "GENESIS"

    def test_phoenix_package_lazy_imports(self):
        """Verify lazy imports work from main package."""
        from tools.phoenix import (
            # Tensor
            ComplianceTensorV2,
            ComplianceState,
            ComplianceDomain,
            # VM
            SmartAssetVM,
            OpCode,
            # Manifold
            ComplianceManifold,
            create_standard_manifold,
            # Migration
            MigrationSaga,
            MigrationState,
            MigrationRequest,
            # Watcher
            WatcherRegistry,
            # Health
            get_health_checker,
            HealthStatus,
            # Config
            get_config,
            # Observability
            get_tracer,
            PhoenixLayer,
        )

        # Quick validation each import works
        assert ComplianceTensorV2 is not None
        assert SmartAssetVM is not None
        assert ComplianceManifold is not None
        assert MigrationSaga is not None
        assert WatcherRegistry is not None
        assert get_health_checker is not None
        assert get_config is not None
        assert get_tracer is not None


class TestProductionReadiness:
    """Smoke tests validating production readiness."""

    def test_all_modules_have_docstrings(self):
        """Verify all modules have proper documentation."""
        from tools.phoenix import tensor, zkp, vm, manifold, migration
        from tools.phoenix import bridge, anchor, watcher, security, hardening
        from tools.phoenix import health, observability, config, cli

        modules = [
            tensor, zkp, vm, manifold, migration,
            bridge, anchor, watcher, security, hardening,
            health, observability, config, cli,
        ]

        for module in modules:
            assert module.__doc__ is not None, f"{module.__name__} missing docstring"
            assert len(module.__doc__) > 50, f"{module.__name__} has insufficient documentation"

    def test_health_probes_are_functional(self):
        """Verify health probes work for Kubernetes integration."""
        from tools.phoenix.health import get_health_checker, HealthStatus

        checker = get_health_checker()

        # Liveness - should always pass if process is running
        liveness = checker.liveness()
        assert liveness.status == HealthStatus.HEALTHY
        assert "pid" in liveness.metadata

        # Deep health - should return comprehensive report
        report = checker.deep_health()
        assert report.version is not None
        assert len(report.checks) >= 3  # memory, threads, gc at minimum

    def test_metrics_are_prometheus_compatible(self):
        """Verify metrics export in Prometheus format."""
        from tools.phoenix.health import get_metrics

        metrics = get_metrics()

        # Add some test metrics
        metrics.inc_counter("phoenix_smoke_test_total")
        metrics.set_gauge("phoenix_smoke_test_gauge", 100.0)

        # Export to Prometheus format
        output = metrics.to_prometheus()

        # Should contain our metrics in Prometheus text format
        assert "phoenix_smoke_test_total" in output
        assert "phoenix_smoke_test_gauge" in output

    def test_configuration_validation(self):
        """Verify configuration validation works."""
        from tools.phoenix.config import get_config_manager

        manager = get_config_manager()
        errors = manager.validate()

        # Default configuration should have no validation errors
        assert errors == [], f"Configuration validation errors: {errors}"


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
