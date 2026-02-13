"""
PHOENIX Event Infrastructure

Event-driven architecture for the Smart Asset Operating System.
Implements typed event bus, event sourcing, and async event processing
for decoupled component communication.

Architecture
────────────

    ┌─────────────────────────────────────────────────────────────────────────┐
    │                          EVENT INFRASTRUCTURE                            │
    │                                                                          │
    │  Event Bus           Event Store        Event Processor                  │
    │  ├─ Typed events     ├─ Append-only     ├─ Handlers                     │
    │  ├─ Sync/async       ├─ Snapshots       ├─ Filters                      │
    │  ├─ Pub/sub          ├─ Replay          ├─ Middleware                   │
    │  └─ Topic routing    └─ Projections     └─ Dead letter                  │
    │                                                                          │
    │  Domain Events       Saga Events        System Events                    │
    │  ├─ AssetCreated     ├─ SagaStarted     ├─ ServiceStarted               │
    │  ├─ AssetMigrated    ├─ StepCompleted   ├─ HealthChanged                │
    │  ├─ ComplianceSet    ├─ SagaCompleted   ├─ ConfigChanged                │
    │  └─ AttestationRcvd  └─ SagaFailed      └─ MetricRecorded               │
    │                                                                          │
    └─────────────────────────────────────────────────────────────────────────┘

Design Principles
─────────────────

    Immutable Events: Events are immutable facts about what happened.
    They cannot be changed, only new events can be appended.

    Eventual Consistency: Subscribers process events asynchronously,
    achieving eventual consistency across components.

    Idempotency: Event handlers should be idempotent to handle
    redelivery scenarios gracefully.

    Ordering: Events within a stream maintain causal ordering.
    Cross-stream ordering is not guaranteed.

Usage
─────

    from tools.phoenix.events import (
        Event,
        EventBus,
        EventStore,
        event_handler,
    )

    # Define domain event
    @dataclass
    class AssetMigrated(Event):
        asset_id: str
        source_jurisdiction: str
        target_jurisdiction: str

    # Subscribe to events
    bus = get_event_bus()

    @bus.subscribe(AssetMigrated)
    def handle_migration(event: AssetMigrated):
        print(f"Asset {event.asset_id} migrated")

    # Publish event
    bus.publish(AssetMigrated(
        asset_id="asset-001",
        source_jurisdiction="uae-difc",
        target_jurisdiction="kz-aifc",
    ))

Copyright (c) 2026 Momentum. All rights reserved.
"""

from __future__ import annotations

import hashlib
import json
import logging
import queue
import threading
import uuid
from abc import ABC, abstractmethod
from dataclasses import dataclass, field, asdict
from datetime import datetime, timezone
from enum import Enum, auto
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

logger = logging.getLogger(__name__)

T = TypeVar("T")
E = TypeVar("E", bound="Event")


# ════════════════════════════════════════════════════════════════════════════
# EVENT BASE
# ════════════════════════════════════════════════════════════════════════════


@dataclass
class Event:
    """
    Base class for all events in the system.

    Events are immutable facts representing something that happened.
    Each event has a unique ID, timestamp, and optional metadata.

    Example:
        @dataclass
        class AssetCreated(Event):
            asset_id: str
            asset_type: str
            owner_did: str

        event = AssetCreated(
            asset_id="asset-001",
            asset_type="token",
            owner_did="did:key:z6Mk...",
        )
    """

    # Metadata fields (auto-populated)
    event_id: str = field(default_factory=lambda: str(uuid.uuid4()))
    event_timestamp: str = field(
        default_factory=lambda: datetime.now(timezone.utc).isoformat()
    )
    correlation_id: Optional[str] = None
    causation_id: Optional[str] = None
    metadata: Dict[str, Any] = field(default_factory=dict)

    @property
    def event_type(self) -> str:
        """Get the event type name."""
        return self.__class__.__name__

    @property
    def timestamp(self) -> datetime:
        """Get timestamp as datetime."""
        return datetime.fromisoformat(self.event_timestamp)

    def to_dict(self) -> Dict[str, Any]:
        """Serialize event to dictionary."""
        data = asdict(self)
        data["event_type"] = self.event_type
        return data

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "Event":
        """Deserialize event from dictionary."""
        data = data.copy()
        data.pop("event_type", None)
        return cls(**data)

    def to_json(self) -> str:
        """Serialize event to JSON for display/transport (not for digest computation)."""
        return json.dumps(self.to_dict(), default=str, sort_keys=True)

    def digest(self) -> str:
        """Compute deterministic digest of event content using JCS canonicalization."""
        from tools.lawpack import jcs_canonicalize
        return hashlib.sha256(jcs_canonicalize(self.to_dict())).hexdigest()


# ════════════════════════════════════════════════════════════════════════════
# DOMAIN EVENTS
# ════════════════════════════════════════════════════════════════════════════


@dataclass
class AssetCreated(Event):
    """Emitted when a new Smart Asset is created."""
    asset_id: str = ""
    asset_type: str = ""
    owner_did: str = ""
    genesis_digest: str = ""


@dataclass
class AssetMigrated(Event):
    """Emitted when a Smart Asset completes migration."""
    asset_id: str = ""
    source_jurisdiction: str = ""
    target_jurisdiction: str = ""
    migration_id: str = ""
    receipt_digest: str = ""


@dataclass
class ComplianceStateChanged(Event):
    """Emitted when compliance state changes in the tensor."""
    asset_id: str = ""
    jurisdiction_id: str = ""
    domain: str = ""
    old_state: str = ""
    new_state: str = ""
    reason_code: str = ""


@dataclass
class AttestationReceived(Event):
    """Emitted when a new attestation is received."""
    attestation_id: str = ""
    attestation_type: str = ""
    subject_id: str = ""
    issuer_did: str = ""
    attestation_digest: str = ""


@dataclass
class MigrationStarted(Event):
    """Emitted when a migration saga begins."""
    migration_id: str = ""
    asset_id: str = ""
    source_jurisdiction: str = ""
    target_jurisdiction: str = ""


@dataclass
class MigrationStepCompleted(Event):
    """Emitted when a migration saga step completes."""
    migration_id: str = ""
    step_name: str = ""
    step_index: int = 0
    evidence_digest: str = ""


@dataclass
class MigrationCompleted(Event):
    """Emitted when a migration saga completes successfully."""
    migration_id: str = ""
    asset_id: str = ""
    total_fees: str = ""
    total_duration_seconds: float = 0.0


@dataclass
class MigrationFailed(Event):
    """Emitted when a migration saga fails."""
    migration_id: str = ""
    asset_id: str = ""
    failure_reason: str = ""
    compensation_required: bool = False


@dataclass
class WatcherSlashed(Event):
    """Emitted when a watcher is slashed."""
    watcher_id: str = ""
    slash_amount: str = ""
    slash_reason: str = ""
    evidence_digest: str = ""


@dataclass
class AnchorSubmitted(Event):
    """Emitted when a checkpoint is anchored to L1."""
    anchor_id: str = ""
    chain: str = ""
    checkpoint_digest: str = ""
    transaction_hash: str = ""


# ════════════════════════════════════════════════════════════════════════════
# EVENT HANDLER
# ════════════════════════════════════════════════════════════════════════════


EventHandler = Callable[[Event], None]


@dataclass
class EventHandlerRegistration:
    """Registration for an event handler."""
    handler: EventHandler
    event_types: Set[Type[Event]]
    priority: int = 0
    filter_func: Optional[Callable[[Event], bool]] = None
    async_handler: bool = False


class EventHandlerError(Exception):
    """Error during event handling."""
    def __init__(self, event: Event, handler: EventHandler, cause: Exception):
        self.event = event
        self.handler = handler
        self.cause = cause
        super().__init__(f"Handler {handler.__name__} failed for {event.event_type}: {cause}")


# ════════════════════════════════════════════════════════════════════════════
# EVENT BUS
# ════════════════════════════════════════════════════════════════════════════


class EventBus:
    """
    In-memory event bus for pub/sub communication.

    Supports typed subscriptions, filters, priorities, and async processing.
    Thread-safe for concurrent publishing and subscribing.

    Example:
        bus = EventBus()

        @bus.subscribe(AssetCreated, AssetMigrated)
        def handle_asset_events(event):
            print(f"Asset event: {event.event_type}")

        bus.publish(AssetCreated(asset_id="001"))
    """

    def __init__(
        self,
        async_queue_size: int = 1000,
        on_error: Optional[Callable[[EventHandlerError], None]] = None,
    ):
        self._handlers: List[EventHandlerRegistration] = []
        self._lock = threading.RLock()
        self._async_queue: queue.Queue = queue.Queue(maxsize=async_queue_size)
        self._async_worker: Optional[threading.Thread] = None
        self._running = False
        self._on_error = on_error
        self._published_count = 0
        self._handled_count = 0
        self._error_count = 0

    def subscribe(
        self,
        *event_types: Type[Event],
        priority: int = 0,
        filter_func: Optional[Callable[[Event], bool]] = None,
        async_handler: bool = False,
    ) -> Callable[[EventHandler], EventHandler]:
        """
        Decorator to subscribe a handler to event types.

        Args:
            event_types: Event types to subscribe to
            priority: Handler priority (higher = earlier)
            filter_func: Optional filter function
            async_handler: Process asynchronously

        Example:
            @bus.subscribe(AssetCreated, priority=10)
            def handle_high_priority(event):
                pass
        """
        def decorator(handler: EventHandler) -> EventHandler:
            registration = EventHandlerRegistration(
                handler=handler,
                event_types=set(event_types) if event_types else {Event},
                priority=priority,
                filter_func=filter_func,
                async_handler=async_handler,
            )
            with self._lock:
                self._handlers.append(registration)
                self._handlers.sort(key=lambda r: -r.priority)
            return handler
        return decorator

    def unsubscribe(self, handler: EventHandler) -> bool:
        """Unsubscribe a handler."""
        with self._lock:
            original_len = len(self._handlers)
            self._handlers = [r for r in self._handlers if r.handler != handler]
            return len(self._handlers) < original_len

    def publish(self, event: Event) -> None:
        """
        Publish an event to all subscribers.

        Synchronous handlers are called immediately in priority order.
        Async handlers are queued for background processing.
        """
        with self._lock:
            self._published_count += 1
            handlers_to_call = []

            for registration in self._handlers:
                # Check if handler subscribes to this event type
                if not any(isinstance(event, t) for t in registration.event_types):
                    continue

                # Check filter
                if registration.filter_func and not registration.filter_func(event):
                    continue

                handlers_to_call.append(registration)

        # Call handlers (outside lock)
        for registration in handlers_to_call:
            if registration.async_handler:
                try:
                    self._async_queue.put_nowait((registration.handler, event))
                except queue.Full:
                    # Queue full - handle synchronously as fallback
                    self._call_handler(registration.handler, event)
            else:
                self._call_handler(registration.handler, event)

    def _call_handler(self, handler: EventHandler, event: Event) -> None:
        """Call a handler with error handling."""
        try:
            handler(event)
            with self._lock:
                self._handled_count += 1
        except Exception as e:
            with self._lock:
                self._error_count += 1
            error = EventHandlerError(event, handler, e)
            if self._on_error:
                self._on_error(error)

    def start_async_processing(self) -> None:
        """Start background thread for async handlers."""
        if self._running:
            return

        self._running = True
        self._async_worker = threading.Thread(
            target=self._async_processor,
            daemon=True,
            name="event-bus-async",
        )
        self._async_worker.start()

    def stop_async_processing(self, timeout: float = 5.0) -> None:
        """Stop background processing."""
        self._running = False
        if self._async_worker:
            self._async_worker.join(timeout=timeout)
            self._async_worker = None

    def _async_processor(self) -> None:
        """Background processor for async handlers."""
        while self._running:
            try:
                handler, event = self._async_queue.get(timeout=0.1)
                self._call_handler(handler, event)
            except queue.Empty:
                continue

    @property
    def metrics(self) -> Dict[str, int]:
        """Get event bus metrics."""
        with self._lock:
            return {
                "published_count": self._published_count,
                "handled_count": self._handled_count,
                "error_count": self._error_count,
                "handler_count": len(self._handlers),
                "async_queue_size": self._async_queue.qsize(),
            }


# ════════════════════════════════════════════════════════════════════════════
# EVENT STORE
# ════════════════════════════════════════════════════════════════════════════


@dataclass
class EventRecord:
    """A persisted event record."""
    sequence_number: int
    event: Event
    stream_id: str
    version: int
    recorded_at: str = field(
        default_factory=lambda: datetime.now(timezone.utc).isoformat()
    )

    def to_dict(self) -> Dict[str, Any]:
        """Serialize to dictionary."""
        return {
            "sequence_number": self.sequence_number,
            "event": self.event.to_dict(),
            "stream_id": self.stream_id,
            "version": self.version,
            "recorded_at": self.recorded_at,
        }


class EventStore:
    """
    Append-only event store for event sourcing.

    Events are organized into streams by aggregate ID.
    Supports replay, snapshots, and optimistic concurrency.

    Example:
        store = EventStore()

        # Append events to a stream
        store.append("asset-001", [
            AssetCreated(asset_id="asset-001"),
            ComplianceStateChanged(asset_id="asset-001", ...),
        ])

        # Replay stream
        events = store.read_stream("asset-001")

        # Get events since position
        events = store.read_all(from_position=100)
    """

    def __init__(self):
        self._events: List[EventRecord] = []
        self._streams: Dict[str, List[EventRecord]] = {}
        self._sequence_number = 0
        self._lock = threading.RLock()

    def append(
        self,
        stream_id: str,
        events: List[Event],
        expected_version: Optional[int] = None,
    ) -> List[EventRecord]:
        """
        Append events to a stream.

        Args:
            stream_id: Stream identifier (usually aggregate ID)
            events: Events to append
            expected_version: For optimistic concurrency (None = no check)

        Returns:
            List of recorded events with sequence numbers

        Raises:
            ConcurrencyError: If expected_version doesn't match
        """
        with self._lock:
            if stream_id not in self._streams:
                self._streams[stream_id] = []

            stream = self._streams[stream_id]
            current_version = len(stream)

            if expected_version is not None and expected_version != current_version:
                raise ConcurrencyError(stream_id, expected_version, current_version)

            records = []
            for event in events:
                self._sequence_number += 1
                record = EventRecord(
                    sequence_number=self._sequence_number,
                    event=event,
                    stream_id=stream_id,
                    version=current_version + 1,
                )
                self._events.append(record)
                stream.append(record)
                records.append(record)
                current_version += 1

            return records

    def read_stream(
        self,
        stream_id: str,
        from_version: int = 0,
        to_version: Optional[int] = None,
    ) -> List[Event]:
        """Read events from a stream."""
        with self._lock:
            if stream_id not in self._streams:
                return []

            stream = self._streams[stream_id]
            to_version = to_version or len(stream)

            return [r.event for r in stream[from_version:to_version]]

    def read_all(
        self,
        from_position: int = 0,
        max_count: int = 1000,
    ) -> List[EventRecord]:
        """Read events from all streams."""
        with self._lock:
            return self._events[from_position:from_position + max_count]

    def get_stream_version(self, stream_id: str) -> int:
        """Get current version of a stream."""
        with self._lock:
            if stream_id not in self._streams:
                return 0
            return len(self._streams[stream_id])

    def get_stream_ids(self) -> List[str]:
        """Get all stream IDs."""
        with self._lock:
            return list(self._streams.keys())

    @property
    def total_events(self) -> int:
        """Total number of events."""
        with self._lock:
            return len(self._events)

    @property
    def current_position(self) -> int:
        """Current global position."""
        with self._lock:
            return self._sequence_number


class ConcurrencyError(Exception):
    """Optimistic concurrency violation."""
    def __init__(self, stream_id: str, expected: int, actual: int):
        self.stream_id = stream_id
        self.expected = expected
        self.actual = actual
        super().__init__(
            f"Concurrency error for stream '{stream_id}': "
            f"expected version {expected}, actual {actual}"
        )


# ════════════════════════════════════════════════════════════════════════════
# EVENT SOURCED AGGREGATE
# ════════════════════════════════════════════════════════════════════════════


class EventSourcedAggregate(ABC):
    """
    Base class for event-sourced aggregates.

    Aggregates encapsulate domain logic and maintain consistency.
    State is derived entirely from the event history.

    Example:
        class SmartAsset(EventSourcedAggregate):
            def __init__(self):
                super().__init__()
                self.asset_id = None
                self.compliance_state = {}

            def _apply_event(self, event: Event):
                if isinstance(event, AssetCreated):
                    self.asset_id = event.asset_id
                elif isinstance(event, ComplianceStateChanged):
                    key = (event.jurisdiction_id, event.domain)
                    self.compliance_state[key] = event.new_state

            def create(self, asset_id: str, asset_type: str, owner_did: str):
                self._raise_event(AssetCreated(
                    asset_id=asset_id,
                    asset_type=asset_type,
                    owner_did=owner_did,
                ))
    """

    def __init__(self):
        self._uncommitted_events: List[Event] = []
        self._version = 0

    @property
    def version(self) -> int:
        """Current aggregate version."""
        return self._version

    @property
    def uncommitted_events(self) -> List[Event]:
        """Events not yet persisted."""
        return self._uncommitted_events.copy()

    def clear_uncommitted_events(self) -> List[Event]:
        """Clear and return uncommitted events."""
        events = self._uncommitted_events
        self._uncommitted_events = []
        return events

    def load_from_history(self, events: List[Event]) -> None:
        """Reconstruct state from event history."""
        for event in events:
            self._apply_event(event)
            self._version += 1

    def _raise_event(self, event: Event) -> None:
        """Raise a new event (command handler)."""
        self._apply_event(event)
        self._uncommitted_events.append(event)
        self._version += 1

    @abstractmethod
    def _apply_event(self, event: Event) -> None:
        """Apply event to update state (projection)."""
        pass


# ════════════════════════════════════════════════════════════════════════════
# PROJECTION
# ════════════════════════════════════════════════════════════════════════════


class Projection(ABC):
    """
    Base class for event projections (read models).

    Projections transform events into query-optimized views.
    They can be rebuilt from the event store at any time.

    Example:
        class AssetCountByJurisdiction(Projection):
            def __init__(self):
                self.counts = {}

            def handle_event(self, event: Event):
                if isinstance(event, AssetMigrated):
                    self.counts[event.source_jurisdiction] = \
                        self.counts.get(event.source_jurisdiction, 0) - 1
                    self.counts[event.target_jurisdiction] = \
                        self.counts.get(event.target_jurisdiction, 0) + 1
    """

    def __init__(self):
        self._position = 0

    @property
    def position(self) -> int:
        """Last processed event position."""
        return self._position

    @abstractmethod
    def handle_event(self, event: Event) -> None:
        """Process an event to update the projection."""
        pass

    def process_events(self, events: List[EventRecord]) -> None:
        """Process a batch of events."""
        for record in events:
            self.handle_event(record.event)
            self._position = record.sequence_number

    def rebuild(self, store: EventStore) -> None:
        """Rebuild projection from event store."""
        self._position = 0
        events = store.read_all(from_position=0, max_count=100000)
        self.process_events(events)


# ════════════════════════════════════════════════════════════════════════════
# EVENT PROCESSOR
# ════════════════════════════════════════════════════════════════════════════


class EventProcessor:
    """
    Processes events from store and dispatches to projections.

    Supports catch-up subscription for replay and real-time updates.

    Example:
        processor = EventProcessor(store, bus)
        processor.add_projection(asset_count_projection)
        processor.start()  # Start processing
    """

    def __init__(
        self,
        store: EventStore,
        bus: Optional[EventBus] = None,
        poll_interval_seconds: float = 0.1,
    ):
        self._store = store
        self._bus = bus
        self._projections: List[Projection] = []
        self._position = 0
        self._running = False
        self._worker: Optional[threading.Thread] = None
        self._poll_interval = poll_interval_seconds
        self._lock = threading.Lock()

    def add_projection(self, projection: Projection) -> None:
        """Add a projection to process."""
        with self._lock:
            self._projections.append(projection)

    def start(self, from_position: int = 0) -> None:
        """Start processing events."""
        if self._running:
            return

        self._position = from_position
        self._running = True
        self._worker = threading.Thread(
            target=self._process_loop,
            daemon=True,
            name="event-processor",
        )
        self._worker.start()

    def stop(self, timeout: float = 5.0) -> None:
        """Stop processing."""
        self._running = False
        if self._worker:
            self._worker.join(timeout=timeout)
            self._worker = None

    def _process_loop(self) -> None:
        """Main processing loop."""
        import time

        while self._running:
            events = self._store.read_all(
                from_position=self._position,
                max_count=100,
            )

            if events:
                with self._lock:
                    for projection in self._projections:
                        projection.process_events(events)

                if self._bus:
                    for record in events:
                        self._bus.publish(record.event)

                self._position = events[-1].sequence_number

            time.sleep(self._poll_interval)

    @property
    def current_position(self) -> int:
        """Current processing position."""
        return self._position


# ════════════════════════════════════════════════════════════════════════════
# SAGA COORDINATOR
# ════════════════════════════════════════════════════════════════════════════


class SagaState(Enum):
    """Saga execution states."""
    PENDING = auto()
    RUNNING = auto()
    COMPLETED = auto()
    COMPENSATING = auto()
    FAILED = auto()


@dataclass
class SagaStep:
    """A step in a saga."""
    name: str
    action: Callable[[], None]
    compensate: Optional[Callable[[], None]] = None


class Saga:
    """
    Saga pattern for distributed transactions.

    Executes a series of steps with compensation on failure.
    Integrates with event bus for progress tracking.

    Example:
        saga = Saga("migration-001", bus)
        saga.add_step("lock_source", lock_asset, unlock_asset)
        saga.add_step("transfer", execute_transfer, reverse_transfer)
        saga.add_step("unlock_target", unlock_asset)

        success = saga.execute()
    """

    def __init__(
        self,
        saga_id: str,
        bus: Optional[EventBus] = None,
    ):
        self.saga_id = saga_id
        self._bus = bus
        self._steps: List[SagaStep] = []
        self._completed_steps: List[SagaStep] = []
        self._state = SagaState.PENDING
        self._error: Optional[Exception] = None

    @property
    def state(self) -> SagaState:
        """Current saga state."""
        return self._state

    @property
    def error(self) -> Optional[Exception]:
        """Error if saga failed."""
        return self._error

    def add_step(
        self,
        name: str,
        action: Callable[[], None],
        compensate: Optional[Callable[[], None]] = None,
    ) -> "Saga":
        """Add a step to the saga."""
        self._steps.append(SagaStep(name, action, compensate))
        return self

    def execute(self) -> bool:
        """
        Execute the saga.

        Returns True if successful, False if failed (with compensation).
        """
        self._state = SagaState.RUNNING

        if self._bus:
            self._bus.publish(MigrationStarted(migration_id=self.saga_id))

        for i, step in enumerate(self._steps):
            try:
                step.action()
                self._completed_steps.append(step)

                if self._bus:
                    self._bus.publish(MigrationStepCompleted(
                        migration_id=self.saga_id,
                        step_name=step.name,
                        step_index=i,
                    ))
            except Exception as e:
                self._error = e
                self._state = SagaState.COMPENSATING
                self._compensate()
                self._state = SagaState.FAILED

                if self._bus:
                    self._bus.publish(MigrationFailed(
                        migration_id=self.saga_id,
                        failure_reason=str(e),
                        compensation_required=len(self._completed_steps) > 0,
                    ))
                return False

        self._state = SagaState.COMPLETED

        if self._bus:
            self._bus.publish(MigrationCompleted(
                migration_id=self.saga_id,
            ))

        return True

    def _compensate(self) -> None:
        """Run compensation for completed steps in reverse order."""
        for step in reversed(self._completed_steps):
            if step.compensate:
                try:
                    step.compensate()
                except Exception as exc:
                    logger.error("Saga compensation step %s failed: %s", step.name, exc, exc_info=True)


# ════════════════════════════════════════════════════════════════════════════
# GLOBAL INSTANCES
# ════════════════════════════════════════════════════════════════════════════


_event_bus: Optional[EventBus] = None
_event_store: Optional[EventStore] = None
_event_bus_lock = threading.Lock()
_event_store_lock = threading.Lock()


def get_event_bus() -> EventBus:
    """Get the global event bus instance."""
    global _event_bus
    if _event_bus is None:
        with _event_bus_lock:
            if _event_bus is None:
                _event_bus = EventBus()
    return _event_bus


def get_event_store() -> EventStore:
    """Get the global event store instance."""
    global _event_store
    if _event_store is None:
        with _event_store_lock:
            if _event_store is None:
                _event_store = EventStore()
    return _event_store


# ════════════════════════════════════════════════════════════════════════════
# CONVENIENCE DECORATOR
# ════════════════════════════════════════════════════════════════════════════


def event_handler(
    *event_types: Type[Event],
    priority: int = 0,
    filter_func: Optional[Callable[[Event], bool]] = None,
    async_handler: bool = False,
):
    """
    Decorator to register a function as an event handler.

    Uses the global event bus.

    Example:
        @event_handler(AssetCreated, AssetMigrated)
        def handle_asset_events(event):
            print(f"Received: {event.event_type}")
    """
    bus = get_event_bus()
    return bus.subscribe(
        *event_types,
        priority=priority,
        filter_func=filter_func,
        async_handler=async_handler,
    )


# ════════════════════════════════════════════════════════════════════════════
# MODULE EXPORTS
# ════════════════════════════════════════════════════════════════════════════


__all__ = [
    # Base
    "Event",
    "EventHandler",
    "EventHandlerRegistration",
    "EventHandlerError",
    # Domain Events
    "AssetCreated",
    "AssetMigrated",
    "ComplianceStateChanged",
    "AttestationReceived",
    "MigrationStarted",
    "MigrationStepCompleted",
    "MigrationCompleted",
    "MigrationFailed",
    "WatcherSlashed",
    "AnchorSubmitted",
    # Event Bus
    "EventBus",
    # Event Store
    "EventRecord",
    "EventStore",
    "ConcurrencyError",
    # Aggregate
    "EventSourcedAggregate",
    # Projection
    "Projection",
    # Processor
    "EventProcessor",
    # Saga
    "SagaState",
    "SagaStep",
    "Saga",
    # Global
    "get_event_bus",
    "get_event_store",
    "event_handler",
]
