"""PHOENIX Runtime Kernel (v0.4.44 GENESIS).

The unifying orchestration layer that bootstraps, coordinates, and manages
all PHOENIX subsystems as a single coherent runtime.

This module provides:
- Lifecycle management (startup, shutdown, health)
- Context propagation across all layers
- Unified metrics aggregation
- Component dependency injection
- Graceful degradation coordination

Architecture:
    ┌─────────────────────────────────────────────────────────────┐
    │                    PHOENIX KERNEL                            │
    │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐         │
    │  │  Lifecycle  │  │   Context   │  │   Metrics   │         │
    │  │   Manager   │  │ Propagator  │  │ Aggregator  │         │
    │  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘         │
    │         │                │                │                 │
    │  ┌──────┴────────────────┴────────────────┴──────┐         │
    │  │              Component Registry                │         │
    │  │   Resilience │ Events │ Cache │ Health │ ...  │         │
    │  └───────────────────────────────────────────────┘         │
    └─────────────────────────────────────────────────────────────┘

Usage:
    from tools.phoenix.runtime import PhoenixKernel, get_kernel

    # Initialize and start
    kernel = PhoenixKernel()
    await kernel.start()

    # Use unified context
    async with kernel.request_context() as ctx:
        # All operations within this context share correlation ID,
        # trace spans, and participate in unified metrics

    # Graceful shutdown
    await kernel.shutdown()
"""

from __future__ import annotations

import asyncio
import contextvars
import enum
import logging
import threading
import time
import uuid
from abc import ABC, abstractmethod
from contextlib import asynccontextmanager, contextmanager
from dataclasses import dataclass, field
from datetime import datetime, timedelta
from typing import (
    Any,
    Callable,
    Dict,
    Generic,
    List,
    Optional,
    Set,
    Type,
    TypeVar,
    Union,
)

__version__ = "0.4.44"
__all__ = [
    # Core kernel
    "PhoenixKernel",
    "get_kernel",
    # Lifecycle
    "LifecycleState",
    "LifecycleHook",
    "Component",
    "ComponentHealth",
    # Context
    "RequestContext",
    "get_current_context",
    "propagate_context",
    # Metrics
    "MetricsAggregator",
    "Metric",
    "MetricType",
    "Counter",
    "Gauge",
    "Histogram",
    # Registry
    "ComponentRegistry",
    "ServiceLocator",
    # Decorators
    "kernel_component",
    "with_context",
]

logger = logging.getLogger(__name__)

# =============================================================================
# CONTEXT PROPAGATION
# =============================================================================

# Context variable for request-scoped data
_current_context: contextvars.ContextVar[Optional["RequestContext"]] = (
    contextvars.ContextVar("phoenix_context", default=None)
)


@dataclass
class RequestContext:
    """Request-scoped context that propagates through all PHOENIX layers.

    This context carries:
    - Correlation ID for distributed tracing
    - Trace/span information
    - Request metadata
    - Baggage items for cross-cutting concerns
    """

    correlation_id: str = field(default_factory=lambda: str(uuid.uuid4()))
    trace_id: str = field(default_factory=lambda: str(uuid.uuid4()))
    span_id: str = field(default_factory=lambda: str(uuid.uuid4()))
    parent_span_id: Optional[str] = None
    start_time: float = field(default_factory=time.time)
    baggage: Dict[str, str] = field(default_factory=dict)
    metadata: Dict[str, Any] = field(default_factory=dict)

    def child_span(self, operation: str) -> "RequestContext":
        """Create a child span context."""
        return RequestContext(
            correlation_id=self.correlation_id,
            trace_id=self.trace_id,
            span_id=str(uuid.uuid4()),
            parent_span_id=self.span_id,
            baggage=dict(self.baggage),
            metadata={"operation": operation, "parent_operation": self.metadata.get("operation")},
        )

    def elapsed_ms(self) -> float:
        """Get elapsed time in milliseconds."""
        return (time.time() - self.start_time) * 1000

    def to_headers(self) -> Dict[str, str]:
        """Export context as HTTP headers for propagation."""
        headers = {
            "X-Correlation-ID": self.correlation_id,
            "X-Trace-ID": self.trace_id,
            "X-Span-ID": self.span_id,
        }
        if self.parent_span_id:
            headers["X-Parent-Span-ID"] = self.parent_span_id
        for key, value in self.baggage.items():
            headers[f"X-Baggage-{key}"] = value
        return headers

    @classmethod
    def from_headers(cls, headers: Dict[str, str]) -> "RequestContext":
        """Create context from HTTP headers."""
        baggage = {}
        for key, value in headers.items():
            if key.startswith("X-Baggage-"):
                baggage[key[10:]] = value

        return cls(
            correlation_id=headers.get("X-Correlation-ID", str(uuid.uuid4())),
            trace_id=headers.get("X-Trace-ID", str(uuid.uuid4())),
            span_id=str(uuid.uuid4()),
            parent_span_id=headers.get("X-Span-ID"),
            baggage=baggage,
        )


def get_current_context() -> Optional[RequestContext]:
    """Get the current request context."""
    return _current_context.get()


def propagate_context(ctx: RequestContext) -> contextvars.Token:
    """Set the current context and return a token for restoration."""
    return _current_context.set(ctx)


@contextmanager
def request_scope(ctx: Optional[RequestContext] = None):
    """Context manager for request-scoped operations.

    Usage:
        with request_scope() as ctx:
            # All operations here share the same context
            result = do_something()
    """
    if ctx is None:
        ctx = RequestContext()
    token = propagate_context(ctx)
    try:
        yield ctx
    finally:
        _current_context.reset(token)


@asynccontextmanager
async def async_request_scope(ctx: Optional[RequestContext] = None):
    """Async context manager for request-scoped operations."""
    if ctx is None:
        ctx = RequestContext()
    token = propagate_context(ctx)
    try:
        yield ctx
    finally:
        _current_context.reset(token)


# =============================================================================
# METRICS AGGREGATION
# =============================================================================


class MetricType(enum.Enum):
    """Types of metrics supported."""

    COUNTER = "counter"
    GAUGE = "gauge"
    HISTOGRAM = "histogram"
    SUMMARY = "summary"


@dataclass
class Metric:
    """Base metric with labels."""

    name: str
    type: MetricType
    description: str
    labels: Dict[str, str] = field(default_factory=dict)
    value: float = 0.0
    timestamp: float = field(default_factory=time.time)


class Counter:
    """Monotonically increasing counter."""

    def __init__(self, name: str, description: str = "", labels: Optional[Dict[str, str]] = None):
        self.name = name
        self.description = description
        self.labels = labels or {}
        self._value = 0.0
        self._lock = threading.Lock()

    def inc(self, value: float = 1.0) -> None:
        """Increment the counter."""
        with self._lock:
            self._value += value

    def get(self) -> float:
        """Get current value."""
        return self._value

    def to_metric(self) -> Metric:
        """Convert to Metric dataclass."""
        return Metric(
            name=self.name,
            type=MetricType.COUNTER,
            description=self.description,
            labels=self.labels,
            value=self._value,
        )


class Gauge:
    """Metric that can go up or down."""

    def __init__(self, name: str, description: str = "", labels: Optional[Dict[str, str]] = None):
        self.name = name
        self.description = description
        self.labels = labels or {}
        self._value = 0.0
        self._lock = threading.Lock()

    def set(self, value: float) -> None:
        """Set the gauge value."""
        with self._lock:
            self._value = value

    def inc(self, value: float = 1.0) -> None:
        """Increment the gauge."""
        with self._lock:
            self._value += value

    def dec(self, value: float = 1.0) -> None:
        """Decrement the gauge."""
        with self._lock:
            self._value -= value

    def get(self) -> float:
        """Get current value."""
        return self._value

    def to_metric(self) -> Metric:
        """Convert to Metric dataclass."""
        return Metric(
            name=self.name,
            type=MetricType.GAUGE,
            description=self.description,
            labels=self.labels,
            value=self._value,
        )


class Histogram:
    """Distribution of values with configurable buckets."""

    DEFAULT_BUCKETS = (0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0)

    def __init__(
        self,
        name: str,
        description: str = "",
        labels: Optional[Dict[str, str]] = None,
        buckets: Optional[tuple] = None,
    ):
        self.name = name
        self.description = description
        self.labels = labels or {}
        self.buckets = buckets or self.DEFAULT_BUCKETS
        self._counts: Dict[float, int] = {b: 0 for b in self.buckets}
        self._counts[float("inf")] = 0
        self._sum = 0.0
        self._count = 0
        self._lock = threading.Lock()

    def observe(self, value: float) -> None:
        """Record an observation."""
        with self._lock:
            self._sum += value
            self._count += 1
            for bucket in self.buckets:
                if value <= bucket:
                    self._counts[bucket] += 1
            self._counts[float("inf")] += 1

    def get_percentile(self, p: float) -> float:
        """Estimate percentile from histogram buckets."""
        if self._count == 0:
            return 0.0
        target = self._count * p
        cumulative = 0
        prev_bucket = 0.0
        for bucket in sorted(self.buckets):
            cumulative += self._counts[bucket]
            if cumulative >= target:
                # Linear interpolation within bucket
                return (prev_bucket + bucket) / 2
            prev_bucket = bucket
        return self.buckets[-1] if self.buckets else 0.0

    def to_metric(self) -> Metric:
        """Convert to Metric dataclass."""
        return Metric(
            name=self.name,
            type=MetricType.HISTOGRAM,
            description=self.description,
            labels=self.labels,
            value=self._sum / self._count if self._count > 0 else 0.0,
        )


class MetricsAggregator:
    """Central aggregator for all PHOENIX metrics.

    Collects metrics from all subsystems and provides unified access.
    """

    _instance: Optional["MetricsAggregator"] = None
    _lock = threading.Lock()

    def __init__(self):
        self._metrics: Dict[str, Union[Counter, Gauge, Histogram]] = {}
        self._subsystem_metrics: Dict[str, List[str]] = {}
        self._lock = threading.Lock()

    @classmethod
    def get_instance(cls) -> "MetricsAggregator":
        """Get singleton instance."""
        if cls._instance is None:
            with cls._lock:
                if cls._instance is None:
                    cls._instance = cls()
        return cls._instance

    def counter(
        self, name: str, description: str = "", labels: Optional[Dict[str, str]] = None, subsystem: str = "kernel"
    ) -> Counter:
        """Get or create a counter."""
        full_name = f"phoenix_{subsystem}_{name}"
        with self._lock:
            if full_name not in self._metrics:
                self._metrics[full_name] = Counter(full_name, description, labels)
                if subsystem not in self._subsystem_metrics:
                    self._subsystem_metrics[subsystem] = []
                self._subsystem_metrics[subsystem].append(full_name)
            return self._metrics[full_name]  # type: ignore

    def gauge(
        self, name: str, description: str = "", labels: Optional[Dict[str, str]] = None, subsystem: str = "kernel"
    ) -> Gauge:
        """Get or create a gauge."""
        full_name = f"phoenix_{subsystem}_{name}"
        with self._lock:
            if full_name not in self._metrics:
                self._metrics[full_name] = Gauge(full_name, description, labels)
                if subsystem not in self._subsystem_metrics:
                    self._subsystem_metrics[subsystem] = []
                self._subsystem_metrics[subsystem].append(full_name)
            return self._metrics[full_name]  # type: ignore

    def histogram(
        self,
        name: str,
        description: str = "",
        labels: Optional[Dict[str, str]] = None,
        buckets: Optional[tuple] = None,
        subsystem: str = "kernel",
    ) -> Histogram:
        """Get or create a histogram."""
        full_name = f"phoenix_{subsystem}_{name}"
        with self._lock:
            if full_name not in self._metrics:
                self._metrics[full_name] = Histogram(full_name, description, labels, buckets)
                if subsystem not in self._subsystem_metrics:
                    self._subsystem_metrics[subsystem] = []
                self._subsystem_metrics[subsystem].append(full_name)
            return self._metrics[full_name]  # type: ignore

    def collect(self) -> List[Metric]:
        """Collect all metrics."""
        return [m.to_metric() for m in self._metrics.values()]

    def collect_subsystem(self, subsystem: str) -> List[Metric]:
        """Collect metrics for a specific subsystem."""
        names = self._subsystem_metrics.get(subsystem, [])
        return [self._metrics[n].to_metric() for n in names if n in self._metrics]

    def prometheus_format(self) -> str:
        """Export metrics in Prometheus text format."""
        lines = []
        for metric in self.collect():
            label_str = ",".join(f'{k}="{v}"' for k, v in metric.labels.items())
            if label_str:
                lines.append(f"# HELP {metric.name} {metric.description}")
                lines.append(f"# TYPE {metric.name} {metric.type.value}")
                lines.append(f"{metric.name}{{{label_str}}} {metric.value}")
            else:
                lines.append(f"# HELP {metric.name} {metric.description}")
                lines.append(f"# TYPE {metric.name} {metric.type.value}")
                lines.append(f"{metric.name} {metric.value}")
        return "\n".join(lines)


# =============================================================================
# LIFECYCLE MANAGEMENT
# =============================================================================


class LifecycleState(enum.Enum):
    """Component lifecycle states."""

    CREATED = "created"
    INITIALIZING = "initializing"
    RUNNING = "running"
    DEGRADED = "degraded"
    STOPPING = "stopping"
    STOPPED = "stopped"
    FAILED = "failed"


@dataclass
class ComponentHealth:
    """Health status of a component."""

    name: str
    state: LifecycleState
    healthy: bool
    message: str = ""
    details: Dict[str, Any] = field(default_factory=dict)
    last_check: float = field(default_factory=time.time)

    def to_dict(self) -> Dict[str, Any]:
        """Convert to dictionary."""
        return {
            "name": self.name,
            "state": self.state.value,
            "healthy": self.healthy,
            "message": self.message,
            "details": self.details,
            "last_check": self.last_check,
        }


class LifecycleHook(ABC):
    """Interface for lifecycle hooks."""

    @abstractmethod
    async def on_start(self) -> None:
        """Called when component starts."""
        pass

    @abstractmethod
    async def on_stop(self) -> None:
        """Called when component stops."""
        pass

    @abstractmethod
    async def health_check(self) -> ComponentHealth:
        """Check component health."""
        pass


T = TypeVar("T")


class Component(Generic[T], ABC):
    """Base class for PHOENIX components with lifecycle management.

    Components automatically register with the kernel and participate
    in coordinated startup/shutdown.
    """

    def __init__(self, name: str, dependencies: Optional[List[str]] = None):
        self.name = name
        self.dependencies = dependencies or []
        self._state = LifecycleState.CREATED
        self._state_lock = threading.Lock()
        self._started_at: Optional[float] = None
        self._stopped_at: Optional[float] = None

    @property
    def state(self) -> LifecycleState:
        """Get current lifecycle state."""
        return self._state

    def _set_state(self, state: LifecycleState) -> None:
        """Set lifecycle state."""
        with self._state_lock:
            old_state = self._state
            self._state = state
            logger.debug(f"Component {self.name}: {old_state.value} -> {state.value}")

    async def start(self) -> None:
        """Start the component."""
        if self._state != LifecycleState.CREATED and self._state != LifecycleState.STOPPED:
            return

        self._set_state(LifecycleState.INITIALIZING)
        try:
            await self._do_start()
            self._started_at = time.time()
            self._set_state(LifecycleState.RUNNING)
        except Exception as e:
            self._set_state(LifecycleState.FAILED)
            logger.error(f"Component {self.name} failed to start: {e}")
            raise

    async def stop(self) -> None:
        """Stop the component."""
        if self._state not in (LifecycleState.RUNNING, LifecycleState.DEGRADED):
            return

        self._set_state(LifecycleState.STOPPING)
        try:
            await self._do_stop()
            self._stopped_at = time.time()
            self._set_state(LifecycleState.STOPPED)
        except Exception as e:
            self._set_state(LifecycleState.FAILED)
            logger.error(f"Component {self.name} failed to stop: {e}")
            raise

    async def health_check(self) -> ComponentHealth:
        """Check component health."""
        if self._state == LifecycleState.RUNNING:
            try:
                return await self._do_health_check()
            except Exception as e:
                return ComponentHealth(
                    name=self.name,
                    state=self._state,
                    healthy=False,
                    message=str(e),
                )
        return ComponentHealth(
            name=self.name,
            state=self._state,
            healthy=self._state == LifecycleState.RUNNING,
            message=f"Component in state: {self._state.value}",
        )

    @abstractmethod
    async def _do_start(self) -> None:
        """Implementation of start logic."""
        pass

    @abstractmethod
    async def _do_stop(self) -> None:
        """Implementation of stop logic."""
        pass

    async def _do_health_check(self) -> ComponentHealth:
        """Implementation of health check. Override for custom checks."""
        return ComponentHealth(
            name=self.name,
            state=self._state,
            healthy=True,
            message="OK",
        )


# =============================================================================
# COMPONENT REGISTRY & SERVICE LOCATOR
# =============================================================================


class ComponentRegistry:
    """Registry for all PHOENIX components.

    Provides dependency-aware component management with ordered
    startup and reverse-order shutdown.
    """

    def __init__(self):
        self._components: Dict[str, Component] = {}
        self._startup_order: List[str] = []
        self._lock = threading.Lock()

    def register(self, component: Component) -> None:
        """Register a component."""
        with self._lock:
            if component.name in self._components:
                raise ValueError(f"Component already registered: {component.name}")
            self._components[component.name] = component
            self._recompute_order()

    def unregister(self, name: str) -> None:
        """Unregister a component."""
        with self._lock:
            if name in self._components:
                del self._components[name]
                self._recompute_order()

    def get(self, name: str) -> Optional[Component]:
        """Get a component by name."""
        return self._components.get(name)

    def get_all(self) -> List[Component]:
        """Get all components in startup order."""
        return [self._components[n] for n in self._startup_order if n in self._components]

    def _recompute_order(self) -> None:
        """Topologically sort components by dependencies."""
        visited: Set[str] = set()
        order: List[str] = []

        def visit(name: str) -> None:
            if name in visited:
                return
            visited.add(name)
            component = self._components.get(name)
            if component:
                for dep in component.dependencies:
                    visit(dep)
                order.append(name)

        for name in self._components:
            visit(name)

        self._startup_order = order

    async def start_all(self) -> Dict[str, ComponentHealth]:
        """Start all components in dependency order."""
        results = {}
        for name in self._startup_order:
            component = self._components.get(name)
            if component:
                try:
                    await component.start()
                    results[name] = await component.health_check()
                except Exception as e:
                    results[name] = ComponentHealth(
                        name=name,
                        state=LifecycleState.FAILED,
                        healthy=False,
                        message=str(e),
                    )
        return results

    async def stop_all(self) -> Dict[str, ComponentHealth]:
        """Stop all components in reverse dependency order."""
        results = {}
        for name in reversed(self._startup_order):
            component = self._components.get(name)
            if component:
                try:
                    await component.stop()
                    results[name] = await component.health_check()
                except Exception as e:
                    results[name] = ComponentHealth(
                        name=name,
                        state=LifecycleState.FAILED,
                        healthy=False,
                        message=str(e),
                    )
        return results

    async def health_check_all(self) -> Dict[str, ComponentHealth]:
        """Health check all components."""
        results = {}
        for name, component in self._components.items():
            results[name] = await component.health_check()
        return results


class ServiceLocator:
    """Service locator for dependency injection.

    Provides type-safe service resolution with lazy initialization.
    """

    def __init__(self):
        self._services: Dict[Type, Any] = {}
        self._factories: Dict[Type, Callable[[], Any]] = {}
        self._lock = threading.Lock()

    def register(self, service_type: Type[T], instance: T) -> None:
        """Register a service instance."""
        with self._lock:
            self._services[service_type] = instance

    def register_factory(self, service_type: Type[T], factory: Callable[[], T]) -> None:
        """Register a service factory for lazy initialization."""
        with self._lock:
            self._factories[service_type] = factory

    def resolve(self, service_type: Type[T]) -> T:
        """Resolve a service by type."""
        with self._lock:
            if service_type in self._services:
                return self._services[service_type]
            if service_type in self._factories:
                instance = self._factories[service_type]()
                self._services[service_type] = instance
                return instance
            raise KeyError(f"Service not registered: {service_type}")

    def try_resolve(self, service_type: Type[T]) -> Optional[T]:
        """Try to resolve a service, returning None if not found."""
        try:
            return self.resolve(service_type)
        except KeyError:
            return None


# =============================================================================
# PHOENIX KERNEL
# =============================================================================


class PhoenixKernel:
    """PHOENIX Runtime Kernel - the unified orchestration layer.

    The kernel is the central coordination point for all PHOENIX subsystems.
    It manages:
    - Component lifecycle (startup, shutdown)
    - Request context propagation
    - Unified metrics aggregation
    - Aggregate health checks
    - Service location

    Usage:
        kernel = PhoenixKernel()
        await kernel.start()

        async with kernel.request_context() as ctx:
            # Operations here share context
            pass

        await kernel.shutdown()
    """

    _instance: Optional["PhoenixKernel"] = None
    _lock = threading.Lock()

    def __init__(self):
        self._state = LifecycleState.CREATED
        self._components = ComponentRegistry()
        self._services = ServiceLocator()
        self._metrics = MetricsAggregator.get_instance()
        self._shutdown_timeout = 30.0  # seconds
        self._startup_time: Optional[float] = None

        # Register core metrics
        self._requests_total = self._metrics.counter(
            "requests_total", "Total number of requests processed", subsystem="kernel"
        )
        self._request_duration = self._metrics.histogram(
            "request_duration_seconds", "Request duration in seconds", subsystem="kernel"
        )
        self._active_requests = self._metrics.gauge("active_requests", "Number of active requests", subsystem="kernel")

    @classmethod
    def get_instance(cls) -> "PhoenixKernel":
        """Get singleton kernel instance."""
        if cls._instance is None:
            with cls._lock:
                if cls._instance is None:
                    cls._instance = cls()
        return cls._instance

    @property
    def state(self) -> LifecycleState:
        """Get kernel state."""
        return self._state

    @property
    def components(self) -> ComponentRegistry:
        """Get component registry."""
        return self._components

    @property
    def services(self) -> ServiceLocator:
        """Get service locator."""
        return self._services

    @property
    def metrics(self) -> MetricsAggregator:
        """Get metrics aggregator."""
        return self._metrics

    @property
    def uptime_seconds(self) -> float:
        """Get kernel uptime in seconds."""
        if self._startup_time is None:
            return 0.0
        return time.time() - self._startup_time

    async def start(self) -> Dict[str, ComponentHealth]:
        """Start the kernel and all registered components."""
        if self._state != LifecycleState.CREATED and self._state != LifecycleState.STOPPED:
            logger.warning(f"Kernel already in state: {self._state.value}")
            return {}

        self._state = LifecycleState.INITIALIZING
        logger.info("PHOENIX Kernel starting...")

        try:
            # Register built-in services
            self._register_builtin_services()

            # Start all components
            results = await self._components.start_all()

            self._startup_time = time.time()
            self._state = LifecycleState.RUNNING
            logger.info("PHOENIX Kernel started successfully")

            return results

        except Exception as e:
            self._state = LifecycleState.FAILED
            logger.error(f"PHOENIX Kernel failed to start: {e}")
            raise

    async def shutdown(self, timeout: Optional[float] = None) -> Dict[str, ComponentHealth]:
        """Gracefully shutdown the kernel and all components."""
        if self._state not in (LifecycleState.RUNNING, LifecycleState.DEGRADED):
            logger.warning(f"Kernel not running: {self._state.value}")
            return {}

        self._state = LifecycleState.STOPPING
        timeout = timeout or self._shutdown_timeout
        logger.info(f"PHOENIX Kernel shutting down (timeout: {timeout}s)...")

        try:
            # Wait for in-flight requests with timeout
            start = time.time()
            while self._active_requests.get() > 0:
                if time.time() - start > timeout:
                    logger.warning("Shutdown timeout reached with active requests")
                    break
                await asyncio.sleep(0.1)

            # Stop all components in reverse order
            results = await self._components.stop_all()

            self._state = LifecycleState.STOPPED
            logger.info("PHOENIX Kernel stopped")

            return results

        except Exception as e:
            self._state = LifecycleState.FAILED
            logger.error(f"PHOENIX Kernel shutdown failed: {e}")
            raise

    async def health(self) -> Dict[str, Any]:
        """Get aggregate health status."""
        component_health = await self._components.health_check_all()

        all_healthy = all(h.healthy for h in component_health.values())
        degraded = any(h.state == LifecycleState.DEGRADED for h in component_health.values())

        if degraded and self._state == LifecycleState.RUNNING:
            self._state = LifecycleState.DEGRADED

        return {
            "status": "healthy" if all_healthy else "degraded" if degraded else "unhealthy",
            "state": self._state.value,
            "uptime_seconds": self.uptime_seconds,
            "components": {name: h.to_dict() for name, h in component_health.items()},
            "timestamp": time.time(),
        }

    @asynccontextmanager
    async def request_context(self, ctx: Optional[RequestContext] = None):
        """Create a request context for tracking operations.

        Usage:
            async with kernel.request_context() as ctx:
                # All operations share this context
                result = await some_operation()
        """
        if ctx is None:
            ctx = RequestContext()

        self._active_requests.inc()
        self._requests_total.inc()
        start = time.time()

        token = propagate_context(ctx)
        try:
            yield ctx
        finally:
            _current_context.reset(token)
            self._active_requests.dec()
            duration = time.time() - start
            self._request_duration.observe(duration)

    @contextmanager
    def sync_request_context(self, ctx: Optional[RequestContext] = None):
        """Synchronous version of request_context."""
        if ctx is None:
            ctx = RequestContext()

        self._active_requests.inc()
        self._requests_total.inc()
        start = time.time()

        token = propagate_context(ctx)
        try:
            yield ctx
        finally:
            _current_context.reset(token)
            self._active_requests.dec()
            duration = time.time() - start
            self._request_duration.observe(duration)

    def _register_builtin_services(self) -> None:
        """Register built-in services."""
        # Register the metrics aggregator
        self._services.register(MetricsAggregator, self._metrics)

        # Lazy registration of subsystem registries
        try:
            from tools.phoenix.resilience import ResilienceRegistry

            self._services.register_factory(ResilienceRegistry, ResilienceRegistry.get_instance)
        except ImportError:
            pass

        try:
            from tools.phoenix.cache import CacheRegistry

            self._services.register_factory(CacheRegistry, CacheRegistry.get_instance)
        except ImportError:
            pass

        try:
            from tools.phoenix.config import ConfigManager

            self._services.register_factory(ConfigManager, ConfigManager.get_instance)
        except ImportError:
            pass


# =============================================================================
# DECORATORS
# =============================================================================


def kernel_component(
    name: str,
    dependencies: Optional[List[str]] = None,
):
    """Decorator to register a class as a kernel component.

    Usage:
        @kernel_component("my_service", dependencies=["config"])
        class MyService(Component):
            async def _do_start(self):
                ...
    """

    def decorator(cls: Type[Component]) -> Type[Component]:
        original_init = cls.__init__

        def new_init(self, *args, **kwargs):
            original_init(self, *args, **kwargs)
            self.name = name
            self.dependencies = dependencies or []
            # Auto-register with kernel
            kernel = PhoenixKernel.get_instance()
            kernel.components.register(self)

        cls.__init__ = new_init
        return cls

    return decorator


def with_context(func: Callable) -> Callable:
    """Decorator to ensure function runs within a request context.

    If no context exists, creates a new one.

    Usage:
        @with_context
        def my_function():
            ctx = get_current_context()
            # ctx is guaranteed to exist
    """

    def wrapper(*args, **kwargs):
        ctx = get_current_context()
        if ctx is None:
            with request_scope() as ctx:
                return func(*args, **kwargs)
        return func(*args, **kwargs)

    return wrapper


# =============================================================================
# MODULE-LEVEL FUNCTIONS
# =============================================================================


def get_kernel() -> PhoenixKernel:
    """Get the PHOENIX kernel singleton."""
    return PhoenixKernel.get_instance()


def get_metrics() -> MetricsAggregator:
    """Get the metrics aggregator singleton."""
    return MetricsAggregator.get_instance()
