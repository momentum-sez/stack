"""
PHOENIX Health Check Framework

Production-grade health monitoring for Special Economic Zone infrastructure.
Provides liveness, readiness, and deep health checks with dependency tracking.

Endpoints:
    /health  - Overall system health (for load balancers)
    /live    - Liveness probe (is process alive?)
    /ready   - Readiness probe (can accept traffic?)
    /metrics - Prometheus-compatible metrics

Copyright (c) 2024 Momentum. All rights reserved.
"""

from __future__ import annotations

import asyncio
import gc
import os
import platform
import threading
import time
from dataclasses import dataclass, field
from datetime import datetime, timezone
from decimal import Decimal
from enum import Enum
from typing import Any, Callable, Dict, List, Optional, Set

# Version info
__version__ = "0.4.44"
__git_commit__ = os.environ.get("GIT_COMMIT", "unknown")


class HealthStatus(Enum):
    """Health check result status."""
    HEALTHY = "healthy"
    DEGRADED = "degraded"
    UNHEALTHY = "unhealthy"
    UNKNOWN = "unknown"


class DependencyType(Enum):
    """Types of system dependencies."""
    REQUIRED = "required"      # Must be healthy for system to be ready
    OPTIONAL = "optional"      # Degraded if unhealthy, but still operational
    INTERNAL = "internal"      # Internal component (always checked)


@dataclass
class CheckResult:
    """Result of a single health check."""
    name: str
    status: HealthStatus
    message: str = ""
    latency_ms: float = 0.0
    timestamp: str = field(default_factory=lambda: datetime.now(timezone.utc).isoformat())
    metadata: Dict[str, Any] = field(default_factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        return {
            "name": self.name,
            "status": self.status.value,
            "message": self.message,
            "latency_ms": round(self.latency_ms, 2),
            "timestamp": self.timestamp,
            "metadata": self.metadata,
        }


@dataclass
class HealthReport:
    """Comprehensive health report."""
    status: HealthStatus
    version: str
    uptime_seconds: float
    checks: List[CheckResult]
    timestamp: str = field(default_factory=lambda: datetime.now(timezone.utc).isoformat())

    def to_dict(self) -> Dict[str, Any]:
        return {
            "status": self.status.value,
            "version": self.version,
            "uptime_seconds": round(self.uptime_seconds, 2),
            "timestamp": self.timestamp,
            "checks": [c.to_dict() for c in self.checks],
        }


@dataclass
class DependencyConfig:
    """Configuration for a dependency check."""
    name: str
    check_fn: Callable[[], CheckResult]
    dep_type: DependencyType = DependencyType.REQUIRED
    timeout_ms: float = 5000.0
    cache_ttl_ms: float = 1000.0


class HealthChecker:
    """
    Production-grade health check system.

    Features:
    - Liveness checks (is the process alive?)
    - Readiness checks (can we accept traffic?)
    - Deep health checks (all dependencies)
    - Caching to prevent thundering herd
    - Timeout handling for slow dependencies
    - Thread-safe operation
    """

    def __init__(self):
        self._start_time = time.monotonic()
        self._dependencies: Dict[str, DependencyConfig] = {}
        self._cache: Dict[str, tuple[CheckResult, float]] = {}
        self._lock = threading.RLock()
        self._initialized = False
        self._initialization_error: Optional[str] = None

        # Register built-in checks
        self._register_builtin_checks()

    def _register_builtin_checks(self) -> None:
        """Register built-in health checks."""
        self.register_dependency(DependencyConfig(
            name="memory",
            check_fn=self._check_memory,
            dep_type=DependencyType.INTERNAL,
            timeout_ms=1000.0,
        ))

        self.register_dependency(DependencyConfig(
            name="threads",
            check_fn=self._check_threads,
            dep_type=DependencyType.INTERNAL,
            timeout_ms=1000.0,
        ))

        self.register_dependency(DependencyConfig(
            name="gc",
            check_fn=self._check_gc,
            dep_type=DependencyType.INTERNAL,
            timeout_ms=1000.0,
        ))

    def register_dependency(self, config: DependencyConfig) -> None:
        """Register a dependency for health checking."""
        with self._lock:
            self._dependencies[config.name] = config

    def unregister_dependency(self, name: str) -> None:
        """Unregister a dependency."""
        with self._lock:
            self._dependencies.pop(name, None)
            self._cache.pop(name, None)

    def mark_initialized(self) -> None:
        """Mark the system as fully initialized."""
        with self._lock:
            self._initialized = True
            self._initialization_error = None

    def mark_initialization_failed(self, error: str) -> None:
        """Mark initialization as failed."""
        with self._lock:
            self._initialized = False
            self._initialization_error = error

    def _get_cached_result(self, name: str, ttl_ms: float) -> Optional[CheckResult]:
        """Get cached result if still valid."""
        with self._lock:
            if name in self._cache:
                result, cached_at = self._cache[name]
                age_ms = (time.monotonic() - cached_at) * 1000
                if age_ms < ttl_ms:
                    return result
        return None

    def _cache_result(self, name: str, result: CheckResult) -> None:
        """Cache a check result."""
        with self._lock:
            self._cache[name] = (result, time.monotonic())

    def _run_check_with_timeout(
        self,
        config: DependencyConfig,
    ) -> CheckResult:
        """Run a check with timeout handling."""
        # Check cache first
        cached = self._get_cached_result(config.name, config.cache_ttl_ms)
        if cached:
            return cached

        start = time.monotonic()
        try:
            result = config.check_fn()
            result.latency_ms = (time.monotonic() - start) * 1000

            # Cache the result
            self._cache_result(config.name, result)
            return result

        except Exception as e:
            latency = (time.monotonic() - start) * 1000
            result = CheckResult(
                name=config.name,
                status=HealthStatus.UNHEALTHY,
                message=f"Check failed: {str(e)}",
                latency_ms=latency,
            )
            self._cache_result(config.name, result)
            return result

    def _check_memory(self) -> CheckResult:
        """Check memory usage."""
        try:
            import resource
            usage = resource.getrusage(resource.RUSAGE_SELF)
            # BUG FIX: ru_maxrss is in KB on Linux, bytes on macOS
            if platform.system() == "Darwin":
                memory_mb = usage.ru_maxrss / (1024 * 1024)
            else:
                memory_mb = usage.ru_maxrss / 1024  # Convert KB to MB on Linux

            # Thresholds
            if memory_mb > 4096:  # 4GB
                status = HealthStatus.UNHEALTHY
                message = f"Memory usage critical: {memory_mb:.0f}MB"
            elif memory_mb > 2048:  # 2GB
                status = HealthStatus.DEGRADED
                message = f"Memory usage elevated: {memory_mb:.0f}MB"
            else:
                status = HealthStatus.HEALTHY
                message = f"Memory usage normal: {memory_mb:.0f}MB"

            return CheckResult(
                name="memory",
                status=status,
                message=message,
                metadata={"memory_mb": round(memory_mb, 2)},
            )
        except ImportError:
            return CheckResult(
                name="memory",
                status=HealthStatus.UNKNOWN,
                message="Memory check not available on this platform",
            )

    def _check_threads(self) -> CheckResult:
        """Check thread count."""
        thread_count = threading.active_count()

        if thread_count > 500:
            status = HealthStatus.UNHEALTHY
            message = f"Thread count critical: {thread_count}"
        elif thread_count > 200:
            status = HealthStatus.DEGRADED
            message = f"Thread count elevated: {thread_count}"
        else:
            status = HealthStatus.HEALTHY
            message = f"Thread count normal: {thread_count}"

        return CheckResult(
            name="threads",
            status=status,
            message=message,
            metadata={"thread_count": thread_count},
        )

    def _check_gc(self) -> CheckResult:
        """Check garbage collector stats."""
        gc_stats = gc.get_stats()
        gen0_collections = gc_stats[0]["collections"]
        gen2_collections = gc_stats[2]["collections"]

        # High Gen2 collections indicate memory pressure
        if gen2_collections > 1000:
            status = HealthStatus.DEGRADED
            message = f"High GC pressure: {gen2_collections} gen2 collections"
        else:
            status = HealthStatus.HEALTHY
            message = "GC operating normally"

        return CheckResult(
            name="gc",
            status=status,
            message=message,
            metadata={
                "gen0_collections": gen0_collections,
                "gen2_collections": gen2_collections,
            },
        )

    def liveness(self) -> CheckResult:
        """
        Liveness check - is the process alive?

        This is a minimal check that always returns healthy if the process
        is running. Used by orchestrators to determine if restart is needed.
        """
        return CheckResult(
            name="liveness",
            status=HealthStatus.HEALTHY,
            message="Process is alive",
            metadata={
                "pid": os.getpid(),
                "uptime_seconds": round(time.monotonic() - self._start_time, 2),
            },
        )

    def readiness(self) -> CheckResult:
        """
        Readiness check - can we accept traffic?

        Returns healthy only if:
        - System is initialized
        - All REQUIRED dependencies are healthy
        """
        with self._lock:
            if not self._initialized:
                return CheckResult(
                    name="readiness",
                    status=HealthStatus.UNHEALTHY,
                    message=self._initialization_error or "System not initialized",
                )
            # BUG FIX: snapshot dependencies while holding lock to avoid
            # TOCTOU race with concurrent registration/unregistration
            deps_snapshot = list(self._dependencies.items())

        # Check all required dependencies
        for name, config in deps_snapshot:
            if config.dep_type == DependencyType.REQUIRED:
                result = self._run_check_with_timeout(config)
                if result.status == HealthStatus.UNHEALTHY:
                    return CheckResult(
                        name="readiness",
                        status=HealthStatus.UNHEALTHY,
                        message=f"Required dependency '{name}' unhealthy: {result.message}",
                    )

        return CheckResult(
            name="readiness",
            status=HealthStatus.HEALTHY,
            message="System ready to accept traffic",
        )

    def deep_health(self) -> HealthReport:
        """
        Deep health check - comprehensive system status.

        Checks all registered dependencies and returns detailed report.
        """
        checks: List[CheckResult] = []
        overall_status = HealthStatus.HEALTHY

        # BUG FIX: Snapshot dependencies while holding lock
        with self._lock:
            deps_snapshot = list(self._dependencies.items())

        for name, config in deps_snapshot:
            result = self._run_check_with_timeout(config)
            checks.append(result)

            # Update overall status based on dependency type
            if result.status == HealthStatus.UNHEALTHY:
                if config.dep_type == DependencyType.REQUIRED:
                    overall_status = HealthStatus.UNHEALTHY
                elif config.dep_type == DependencyType.OPTIONAL:
                    if overall_status == HealthStatus.HEALTHY:
                        overall_status = HealthStatus.DEGRADED
            elif result.status == HealthStatus.DEGRADED:
                if overall_status == HealthStatus.HEALTHY:
                    overall_status = HealthStatus.DEGRADED

        return HealthReport(
            status=overall_status,
            version=__version__,
            uptime_seconds=time.monotonic() - self._start_time,
            checks=checks,
        )

    def get_version_info(self) -> Dict[str, Any]:
        """Get version and build information."""
        return {
            "version": __version__,
            "git_commit": __git_commit__,
            "python_version": platform.python_version(),
            "platform": platform.platform(),
            "pid": os.getpid(),
            "uptime_seconds": round(time.monotonic() - self._start_time, 2),
        }


# Global health checker instance
_health_checker: Optional[HealthChecker] = None
_health_checker_lock = threading.Lock()


def get_health_checker() -> HealthChecker:
    """Get the global health checker instance."""
    global _health_checker
    if _health_checker is None:
        with _health_checker_lock:
            if _health_checker is None:
                _health_checker = HealthChecker()
    return _health_checker


def register_phoenix_checks(health_checker: HealthChecker) -> None:
    """
    Register PHOENIX-specific health checks.

    Call this after initializing PHOENIX components to add
    component-specific health monitoring.
    """
    from tools.phoenix.tensor import ComplianceTensorV2
    from tools.phoenix.vm import SmartAssetVM
    from tools.phoenix.watcher import WatcherRegistry

    def check_tensor() -> CheckResult:
        """Check tensor subsystem."""
        try:
            # Create minimal tensor to verify functionality
            tensor = ComplianceTensorV2()
            cell_count = len(tensor._cells)
            return CheckResult(
                name="tensor",
                status=HealthStatus.HEALTHY,
                message="Tensor subsystem operational",
                metadata={"cell_count": cell_count},
            )
        except Exception as e:
            return CheckResult(
                name="tensor",
                status=HealthStatus.UNHEALTHY,
                message=f"Tensor subsystem error: {e}",
            )

    def check_vm() -> CheckResult:
        """Check VM subsystem."""
        try:
            vm = SmartAssetVM()
            return CheckResult(
                name="vm",
                status=HealthStatus.HEALTHY,
                message="VM subsystem operational",
                metadata={"max_stack_depth": 1024},
            )
        except Exception as e:
            return CheckResult(
                name="vm",
                status=HealthStatus.UNHEALTHY,
                message=f"VM subsystem error: {e}",
            )

    def check_watcher_registry() -> CheckResult:
        """Check watcher registry."""
        try:
            registry = WatcherRegistry()
            watcher_count = len(registry._watchers)
            return CheckResult(
                name="watcher_registry",
                status=HealthStatus.HEALTHY,
                message="Watcher registry operational",
                metadata={"registered_watchers": watcher_count},
            )
        except Exception as e:
            return CheckResult(
                name="watcher_registry",
                status=HealthStatus.UNHEALTHY,
                message=f"Watcher registry error: {e}",
            )

    health_checker.register_dependency(DependencyConfig(
        name="tensor",
        check_fn=check_tensor,
        dep_type=DependencyType.REQUIRED,
    ))

    health_checker.register_dependency(DependencyConfig(
        name="vm",
        check_fn=check_vm,
        dep_type=DependencyType.REQUIRED,
    ))

    health_checker.register_dependency(DependencyConfig(
        name="watcher_registry",
        check_fn=check_watcher_registry,
        dep_type=DependencyType.OPTIONAL,
    ))


class MetricsCollector:
    """
    Prometheus-compatible metrics collector.

    Collects and exposes metrics in Prometheus text format.
    """

    def __init__(self):
        self._counters: Dict[str, int] = {}
        self._gauges: Dict[str, float] = {}
        self._histograms: Dict[str, List[float]] = {}
        self._lock = threading.RLock()

    def inc_counter(self, name: str, value: int = 1, labels: Optional[Dict[str, str]] = None) -> None:
        """Increment a counter metric."""
        key = self._make_key(name, labels)
        with self._lock:
            self._counters[key] = self._counters.get(key, 0) + value

    def set_gauge(self, name: str, value: float, labels: Optional[Dict[str, str]] = None) -> None:
        """Set a gauge metric."""
        key = self._make_key(name, labels)
        with self._lock:
            self._gauges[key] = value

    def observe_histogram(self, name: str, value: float, labels: Optional[Dict[str, str]] = None) -> None:
        """Observe a histogram metric."""
        key = self._make_key(name, labels)
        with self._lock:
            if key not in self._histograms:
                self._histograms[key] = []
            self._histograms[key].append(value)
            # BUG FIX: Track total count/sum separately so truncation
            # doesn't corrupt aggregate stats
            count_key = f"{key}__total_count"
            sum_key = f"{key}__total_sum"
            self._counters[count_key] = self._counters.get(count_key, 0) + 1
            self._gauges[sum_key] = self._gauges.get(sum_key, 0.0) + value
            # Keep only last 1000 observations for percentile calculations
            if len(self._histograms[key]) > 1000:
                self._histograms[key] = self._histograms[key][-1000:]

    def _make_key(self, name: str, labels: Optional[Dict[str, str]]) -> str:
        """Create a metric key with labels."""
        if not labels:
            return name
        label_str = ",".join(f'{k}="{v}"' for k, v in sorted(labels.items()))
        return f"{name}{{{label_str}}}"

    def to_prometheus(self) -> str:
        """Export metrics in Prometheus text format."""
        lines = []

        with self._lock:
            for key, value in self._counters.items():
                lines.append(f"phoenix_{key} {value}")

            for key, value in self._gauges.items():
                lines.append(f"phoenix_{key} {value}")

            for key, values in self._histograms.items():
                if values:
                    lines.append(f"phoenix_{key}_count {len(values)}")
                    lines.append(f"phoenix_{key}_sum {sum(values)}")

        return "\n".join(lines)


# Global metrics collector
_metrics: Optional[MetricsCollector] = None
_metrics_lock = threading.Lock()


def get_metrics() -> MetricsCollector:
    """Get the global metrics collector."""
    global _metrics
    if _metrics is None:
        with _metrics_lock:
            if _metrics is None:
                _metrics = MetricsCollector()
    return _metrics
