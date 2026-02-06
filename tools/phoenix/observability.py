"""
PHOENIX Observability Framework

Structured logging, tracing, and event emission for production monitoring.
Provides correlation IDs, context propagation, and OpenTelemetry integration.

Architecture:
    ┌─────────────────────────────────────────────────────────┐
    │                    Application Code                      │
    │  logger.info("msg", asset_id=x)  span.record(event)     │
    └───────────────────────┬─────────────────────────────────┘
                            │
    ┌───────────────────────▼─────────────────────────────────┐
    │                 PhoenixLogger / Tracer                   │
    │  Context propagation, correlation IDs, structured data  │
    └───────────────────────┬─────────────────────────────────┘
                            │
    ┌───────────────────────▼─────────────────────────────────┐
    │                    Event Handlers                        │
    │  ConsoleHandler │ FileHandler │ OpenTelemetryExporter   │
    └─────────────────────────────────────────────────────────┘

Copyright (c) 2024 Momentum. All rights reserved.
"""

from __future__ import annotations

import contextvars
import json
import logging
import os
import sys
import threading
import time
import traceback
import uuid
from dataclasses import asdict, dataclass, field
from datetime import datetime, timezone
from enum import Enum
from typing import Any, Callable, Dict, List, Optional, TypeVar, Union

# Context variables for request-scoped data
correlation_id_var: contextvars.ContextVar[str] = contextvars.ContextVar(
    "correlation_id", default=""
)
span_id_var: contextvars.ContextVar[str] = contextvars.ContextVar(
    "span_id", default=""
)
trace_id_var: contextvars.ContextVar[str] = contextvars.ContextVar(
    "trace_id", default=""
)


class LogLevel(Enum):
    """Log severity levels."""
    DEBUG = "debug"
    INFO = "info"
    WARNING = "warning"
    ERROR = "error"
    CRITICAL = "critical"


class PhoenixLayer(Enum):
    """PHOENIX system layers for categorization."""
    TENSOR = "tensor"
    VM = "vm"
    ZK = "zk"
    MANIFOLD = "manifold"
    MIGRATION = "migration"
    BRIDGE = "bridge"
    ANCHOR = "anchor"
    WATCHER = "watcher"
    SECURITY = "security"
    HARDENING = "hardening"
    CONFIG = "config"
    HEALTH = "health"
    CLI = "cli"


@dataclass
class LogEvent:
    """Structured log event."""
    timestamp: str
    level: str
    logger: str
    message: str
    correlation_id: str = ""
    trace_id: str = ""
    span_id: str = ""
    layer: str = ""
    operation: str = ""
    duration_ms: Optional[float] = None
    error_code: str = ""
    context: Dict[str, Any] = field(default_factory=dict)
    exception: Optional[str] = None

    def to_dict(self) -> Dict[str, Any]:
        """Convert to dictionary, excluding None values."""
        d = asdict(self)
        return {k: v for k, v in d.items() if v is not None and v != "" and v != {}}

    def to_json(self) -> str:
        """Convert to JSON string."""
        return json.dumps(self.to_dict(), default=str)


@dataclass
class SpanEvent:
    """Event recorded within a span."""
    name: str
    timestamp: str
    attributes: Dict[str, Any] = field(default_factory=dict)


@dataclass
class Span:
    """
    Distributed tracing span.

    Represents a unit of work within a trace, with timing,
    attributes, and parent-child relationships.
    """
    trace_id: str
    span_id: str
    parent_span_id: str = ""
    name: str = ""
    layer: str = ""
    start_time: float = field(default_factory=time.monotonic)
    end_time: Optional[float] = None
    status: str = "ok"
    attributes: Dict[str, Any] = field(default_factory=dict)
    events: List[SpanEvent] = field(default_factory=list)

    def record_event(self, name: str, **attributes: Any) -> None:
        """Record an event within this span."""
        self.events.append(SpanEvent(
            name=name,
            timestamp=datetime.now(timezone.utc).isoformat(),
            attributes=attributes,
        ))

    def set_attribute(self, key: str, value: Any) -> None:
        """Set a span attribute."""
        self.attributes[key] = value

    def set_status(self, status: str, message: str = "") -> None:
        """Set span status."""
        self.status = status
        if message:
            self.attributes["status_message"] = message

    def end(self) -> None:
        """End the span."""
        self.end_time = time.monotonic()

    @property
    def duration_ms(self) -> float:
        """Get span duration in milliseconds."""
        end = self.end_time or time.monotonic()
        return (end - self.start_time) * 1000

    def to_dict(self) -> Dict[str, Any]:
        """Convert to dictionary."""
        return {
            "trace_id": self.trace_id,
            "span_id": self.span_id,
            "parent_span_id": self.parent_span_id,
            "name": self.name,
            "layer": self.layer,
            "duration_ms": round(self.duration_ms, 2),
            "status": self.status,
            "attributes": self.attributes,
            "events": [asdict(e) for e in self.events],
        }


class SpanContext:
    """Context manager for spans."""

    def __init__(self, tracer: "Tracer", name: str, layer: PhoenixLayer, **attributes: Any):
        self.tracer = tracer
        self.name = name
        self.layer = layer
        self.attributes = attributes
        self.span: Optional[Span] = None
        self._token: Optional[contextvars.Token] = None

    def __enter__(self) -> Span:
        self.span = self.tracer.start_span(self.name, self.layer, **self.attributes)
        self._token = span_id_var.set(self.span.span_id)
        return self.span

    def __exit__(self, exc_type: Any, exc_val: Any, exc_tb: Any) -> None:
        if self.span:
            if exc_type:
                self.span.set_status("error", str(exc_val))
                self.span.set_attribute("exception_type", exc_type.__name__)
                self.span.set_attribute("exception_message", str(exc_val))
            self.span.end()
            self.tracer.end_span(self.span)
        if self._token:
            span_id_var.reset(self._token)


class Tracer:
    """
    Distributed tracing implementation.

    Creates and manages spans for tracking request flow
    across PHOENIX layers.
    """

    def __init__(self, service_name: str = "phoenix"):
        self.service_name = service_name
        self._spans: Dict[str, Span] = {}
        self._lock = threading.RLock()
        self._exporters: List[Callable[[Span], None]] = []

    def add_exporter(self, exporter: Callable[[Span], None]) -> None:
        """Add a span exporter."""
        self._exporters.append(exporter)

    def start_trace(self) -> str:
        """Start a new trace and return trace ID."""
        trace_id = uuid.uuid4().hex
        trace_id_var.set(trace_id)
        return trace_id

    def start_span(
        self,
        name: str,
        layer: PhoenixLayer,
        **attributes: Any,
    ) -> Span:
        """Start a new span."""
        trace_id = trace_id_var.get() or self.start_trace()
        parent_span_id = span_id_var.get()
        span_id = uuid.uuid4().hex[:16]

        span = Span(
            trace_id=trace_id,
            span_id=span_id,
            parent_span_id=parent_span_id,
            name=name,
            layer=layer.value,
            attributes=attributes,
        )

        with self._lock:
            self._spans[span_id] = span

        return span

    def end_span(self, span: Span) -> None:
        """End and export a span."""
        span.end()

        with self._lock:
            self._spans.pop(span.span_id, None)

        # Export to all exporters
        for exporter in self._exporters:
            try:
                exporter(span)
            except Exception:
                pass  # Don't let exporter errors break tracing

    def span(self, name: str, layer: PhoenixLayer, **attributes: Any) -> SpanContext:
        """Create a span context manager."""
        return SpanContext(self, name, layer, **attributes)


class StructuredHandler(logging.Handler):
    """Logging handler that outputs structured JSON."""

    def __init__(self, stream: Any = None):
        super().__init__()
        self.stream = stream or sys.stderr

    def emit(self, record: logging.LogRecord) -> None:
        try:
            event = LogEvent(
                timestamp=datetime.fromtimestamp(record.created, tz=timezone.utc).isoformat(),
                level=record.levelname.lower(),
                logger=record.name,
                message=record.getMessage(),
                correlation_id=correlation_id_var.get(),
                trace_id=trace_id_var.get(),
                span_id=span_id_var.get(),
                layer=getattr(record, "layer", ""),
                operation=getattr(record, "operation", ""),
                duration_ms=getattr(record, "duration_ms", None),
                error_code=getattr(record, "error_code", ""),
                context=getattr(record, "context", {}),
            )

            if record.exc_info:
                event.exception = "".join(traceback.format_exception(*record.exc_info))

            self.stream.write(event.to_json() + "\n")
            self.stream.flush()
        except Exception:
            self.handleError(record)


class PhoenixLogger:
    """
    Structured logger for PHOENIX components.

    Automatically includes correlation IDs, trace context,
    and layer information in all log events.
    """

    def __init__(
        self,
        name: str,
        layer: PhoenixLayer,
        level: LogLevel = LogLevel.INFO,
    ):
        self.name = name
        self.layer = layer
        self._logger = logging.getLogger(f"phoenix.{layer.value}.{name}")
        self._logger.setLevel(getattr(logging, level.value.upper()))

        # Add structured handler if not already added
        if not any(isinstance(h, StructuredHandler) for h in self._logger.handlers):
            handler = StructuredHandler()
            self._logger.addHandler(handler)

    def _log(
        self,
        level: int,
        message: str,
        operation: str = "",
        error_code: str = "",
        duration_ms: Optional[float] = None,
        exc_info: bool = False,
        **context: Any,
    ) -> None:
        """Internal log method."""
        extra = {
            "layer": self.layer.value,
            "operation": operation,
            "error_code": error_code,
            "duration_ms": duration_ms,
            "context": context,
        }
        self._logger.log(level, message, extra=extra, exc_info=exc_info)

    def debug(self, message: str, **context: Any) -> None:
        """Log debug message."""
        self._log(logging.DEBUG, message, **context)

    def info(self, message: str, **context: Any) -> None:
        """Log info message."""
        self._log(logging.INFO, message, **context)

    def warning(self, message: str, **context: Any) -> None:
        """Log warning message."""
        self._log(logging.WARNING, message, **context)

    def error(
        self,
        message: str,
        error_code: str = "",
        exc_info: bool = False,
        **context: Any,
    ) -> None:
        """Log error message."""
        self._log(logging.ERROR, message, error_code=error_code, exc_info=exc_info, **context)

    def critical(
        self,
        message: str,
        error_code: str = "",
        exc_info: bool = False,
        **context: Any,
    ) -> None:
        """Log critical message."""
        self._log(logging.CRITICAL, message, error_code=error_code, exc_info=exc_info, **context)

    def operation(
        self,
        name: str,
        duration_ms: float,
        success: bool = True,
        **context: Any,
    ) -> None:
        """Log an operation completion."""
        level = logging.INFO if success else logging.WARNING
        status = "completed" if success else "failed"
        self._log(
            level,
            f"Operation {name} {status}",
            operation=name,
            duration_ms=duration_ms,
            **context,
        )


def generate_correlation_id() -> str:
    """Generate a new correlation ID."""
    return f"corr-{uuid.uuid4().hex[:12]}"


def set_correlation_id(correlation_id: str) -> contextvars.Token:
    """Set the correlation ID for the current context."""
    return correlation_id_var.set(correlation_id)


def get_correlation_id() -> str:
    """Get the current correlation ID."""
    cid = correlation_id_var.get()
    if not cid:
        cid = generate_correlation_id()
        correlation_id_var.set(cid)
    return cid


# Global tracer instance
_tracer: Optional[Tracer] = None


def get_tracer() -> Tracer:
    """Get the global tracer instance."""
    global _tracer
    if _tracer is None:
        _tracer = Tracer()
    return _tracer


def get_logger(name: str, layer: PhoenixLayer) -> PhoenixLogger:
    """Get a logger for a PHOENIX component."""
    return PhoenixLogger(name, layer)


# Convenience function for timing operations
T = TypeVar("T")


def timed_operation(
    logger: PhoenixLogger,
    operation_name: str,
) -> Callable[[Callable[..., T]], Callable[..., T]]:
    """Decorator for timing and logging operations."""
    def decorator(func: Callable[..., T]) -> Callable[..., T]:
        def wrapper(*args: Any, **kwargs: Any) -> T:
            start = time.monotonic()
            success = True
            try:
                result = func(*args, **kwargs)
                return result
            except Exception:
                success = False
                raise
            finally:
                duration_ms = (time.monotonic() - start) * 1000
                logger.operation(operation_name, duration_ms, success)
        return wrapper
    return decorator


# Audit logging for compliance
@dataclass
class AuditEvent:
    """Audit event for compliance logging."""
    event_id: str
    timestamp: str
    actor_did: str
    action: str
    resource_type: str
    resource_id: str
    outcome: str  # success, failure, denied
    jurisdiction_id: str = ""
    ip_address: str = ""
    user_agent: str = ""
    correlation_id: str = ""
    details: Dict[str, Any] = field(default_factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


class AuditLogger:
    """
    Immutable audit logging for compliance.

    Generates tamper-evident audit trail with hash chaining.
    """

    def __init__(self, logger: PhoenixLogger):
        self._logger = logger
        self._last_hash: str = "genesis"
        self._lock = threading.Lock()

    def _compute_hash(self, event: AuditEvent) -> str:
        """Compute hash for audit event."""
        import hashlib
        data = json.dumps(event.to_dict(), sort_keys=True) + self._last_hash
        return hashlib.sha256(data.encode()).hexdigest()

    def log(
        self,
        actor_did: str,
        action: str,
        resource_type: str,
        resource_id: str,
        outcome: str,
        jurisdiction_id: str = "",
        **details: Any,
    ) -> AuditEvent:
        """Log an audit event."""
        event = AuditEvent(
            event_id=uuid.uuid4().hex,
            timestamp=datetime.now(timezone.utc).isoformat(),
            actor_did=actor_did,
            action=action,
            resource_type=resource_type,
            resource_id=resource_id,
            outcome=outcome,
            jurisdiction_id=jurisdiction_id,
            correlation_id=get_correlation_id(),
            details=details,
        )

        with self._lock:
            event_hash = self._compute_hash(event)
            self._last_hash = event_hash

        self._logger.info(
            f"AUDIT: {action} on {resource_type}/{resource_id}",
            operation="audit",
            **event.to_dict(),
            event_hash=event_hash,
        )

        return event
