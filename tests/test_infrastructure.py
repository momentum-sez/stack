"""
Tests for PHOENIX Infrastructure Patterns

Comprehensive tests for resilience, events, and cache modules.
"""

import pytest
import threading
import time
from datetime import datetime, timezone


# ════════════════════════════════════════════════════════════════════════════
# RESILIENCE TESTS
# ════════════════════════════════════════════════════════════════════════════


class TestCircuitBreaker:
    """Tests for circuit breaker pattern."""

    def test_circuit_starts_closed(self):
        """Circuit breaker starts in CLOSED state."""
        from tools.phoenix.resilience import CircuitBreaker, CircuitState

        breaker = CircuitBreaker("test", failure_threshold=3)
        assert breaker.state == CircuitState.CLOSED

    def test_circuit_opens_after_failures(self):
        """Circuit opens after reaching failure threshold."""
        from tools.phoenix.resilience import CircuitBreaker, CircuitState

        breaker = CircuitBreaker("test", failure_threshold=3)

        for _ in range(3):
            try:
                with breaker:
                    raise ValueError("test error")
            except ValueError:
                pass

        assert breaker.state == CircuitState.OPEN

    def test_open_circuit_rejects_calls(self):
        """Open circuit rejects calls immediately."""
        from tools.phoenix.resilience import CircuitBreaker, CircuitBreakerError

        breaker = CircuitBreaker("test", failure_threshold=1)

        # Trip the breaker
        try:
            with breaker:
                raise ValueError("test")
        except ValueError:
            pass

        # Should reject
        with pytest.raises(CircuitBreakerError):
            with breaker:
                pass

    def test_circuit_metrics_tracking(self):
        """Circuit breaker tracks metrics correctly."""
        from tools.phoenix.resilience import CircuitBreaker

        breaker = CircuitBreaker("test", failure_threshold=5)

        # Successful calls
        for _ in range(3):
            with breaker:
                pass

        metrics = breaker.metrics
        assert metrics.successful_calls == 3
        assert metrics.total_calls == 3

    def test_circuit_decorator(self):
        """Circuit breaker works as decorator."""
        from tools.phoenix.resilience import CircuitBreaker

        breaker = CircuitBreaker("test", failure_threshold=5)

        @breaker
        def success_func():
            return 42

        result = success_func()
        assert result == 42
        assert breaker.metrics.successful_calls == 1


class TestRetryPolicy:
    """Tests for retry policy pattern."""

    def test_retry_succeeds_eventually(self):
        """Retry succeeds after transient failures."""
        from tools.phoenix.resilience import RetryPolicy

        attempt_count = [0]

        def flaky_function():
            attempt_count[0] += 1
            if attempt_count[0] < 3:
                raise ValueError("transient error")
            return "success"

        retry = RetryPolicy(max_attempts=5, base_delay_seconds=0.01)
        result = retry.execute(flaky_function)

        assert result == "success"
        assert attempt_count[0] == 3

    def test_retry_exhausted(self):
        """Retry raises after exhausting attempts."""
        from tools.phoenix.resilience import RetryPolicy, RetryExhaustedError

        def always_fails():
            raise ValueError("always fails")

        retry = RetryPolicy(max_attempts=3, base_delay_seconds=0.01)

        with pytest.raises(RetryExhaustedError) as exc_info:
            retry.execute(always_fails)

        assert exc_info.value.attempts == 3

    def test_retry_decorator(self):
        """Retry works as decorator."""
        from tools.phoenix.resilience import RetryPolicy

        retry = RetryPolicy(max_attempts=3, base_delay_seconds=0.01)
        attempt_count = [0]

        @retry
        def sometimes_fails():
            attempt_count[0] += 1
            if attempt_count[0] < 2:
                raise ValueError("first attempt fails")
            return "success"

        result = sometimes_fails()
        assert result == "success"


class TestBulkhead:
    """Tests for bulkhead pattern."""

    def test_bulkhead_limits_concurrency(self):
        """Bulkhead limits concurrent executions."""
        from tools.phoenix.resilience import Bulkhead

        bulkhead = Bulkhead("test", max_concurrent=2)

        # Should allow up to 2 concurrent
        assert bulkhead.available_permits == 2

        with bulkhead:
            assert bulkhead.available_permits == 1
            with bulkhead:
                assert bulkhead.available_permits == 0

        assert bulkhead.available_permits == 2

    def test_bulkhead_rejects_when_full(self):
        """Bulkhead rejects when at capacity."""
        from tools.phoenix.resilience import Bulkhead, BulkheadFullError

        bulkhead = Bulkhead("test", max_concurrent=1)

        with bulkhead:
            with pytest.raises(BulkheadFullError):
                with bulkhead:
                    pass


class TestTimeout:
    """Tests for timeout pattern."""

    def test_timeout_succeeds_within_limit(self):
        """Timeout allows fast operations."""
        from tools.phoenix.resilience import Timeout

        timeout = Timeout(seconds=1.0)

        @timeout
        def fast_operation():
            return 42

        result = fast_operation()
        assert result == 42


class TestResilient:
    """Tests for composite resilience decorator."""

    def test_resilient_combines_patterns(self):
        """Resilient decorator combines multiple patterns."""
        from tools.phoenix.resilience import (
            resilient,
            CircuitBreaker,
            RetryPolicy,
        )

        breaker = CircuitBreaker("test", failure_threshold=10)
        retry = RetryPolicy(max_attempts=2, base_delay_seconds=0.01)

        call_count = [0]

        @resilient(circuit_breaker=breaker, retry=retry)
        def flaky_service():
            call_count[0] += 1
            if call_count[0] < 2:
                raise ValueError("transient")
            return "success"

        result = flaky_service()
        assert result == "success"
        assert call_count[0] == 2


# ════════════════════════════════════════════════════════════════════════════
# EVENTS TESTS
# ════════════════════════════════════════════════════════════════════════════


class TestEvent:
    """Tests for event base class."""

    def test_event_has_id_and_timestamp(self):
        """Events have auto-generated ID and timestamp."""
        from tools.phoenix.events import Event

        event = Event()
        assert event.event_id is not None
        assert event.event_timestamp is not None

    def test_event_serialization(self):
        """Events serialize to dict and JSON."""
        from tools.phoenix.events import AssetCreated

        event = AssetCreated(asset_id="001", asset_type="token", owner_did="did:key:z6Mk")
        data = event.to_dict()

        assert data["asset_id"] == "001"
        assert data["event_type"] == "AssetCreated"

        json_str = event.to_json()
        assert "001" in json_str


class TestEventBus:
    """Tests for event bus."""

    def test_subscribe_and_publish(self):
        """Event bus delivers events to subscribers."""
        from tools.phoenix.events import EventBus, AssetCreated

        bus = EventBus()
        received = []

        @bus.subscribe(AssetCreated)
        def handler(event):
            received.append(event)

        bus.publish(AssetCreated(asset_id="001"))

        assert len(received) == 1
        assert received[0].asset_id == "001"

    def test_multiple_subscribers(self):
        """Multiple subscribers receive same event."""
        from tools.phoenix.events import EventBus, AssetCreated

        bus = EventBus()
        received1 = []
        received2 = []

        @bus.subscribe(AssetCreated)
        def handler1(event):
            received1.append(event)

        @bus.subscribe(AssetCreated)
        def handler2(event):
            received2.append(event)

        bus.publish(AssetCreated(asset_id="001"))

        assert len(received1) == 1
        assert len(received2) == 1

    def test_event_type_filtering(self):
        """Handlers only receive subscribed event types."""
        from tools.phoenix.events import EventBus, AssetCreated, AssetMigrated

        bus = EventBus()
        received = []

        @bus.subscribe(AssetCreated)
        def handler(event):
            received.append(event)

        bus.publish(AssetCreated(asset_id="001"))
        bus.publish(AssetMigrated(asset_id="002"))

        assert len(received) == 1
        assert received[0].asset_id == "001"


class TestEventStore:
    """Tests for event store."""

    def test_append_and_read(self):
        """Event store appends and reads events."""
        from tools.phoenix.events import EventStore, AssetCreated

        store = EventStore()

        store.append("asset-001", [
            AssetCreated(asset_id="001"),
            AssetCreated(asset_id="002"),
        ])

        events = store.read_stream("asset-001")
        assert len(events) == 2

    def test_optimistic_concurrency(self):
        """Event store enforces optimistic concurrency."""
        from tools.phoenix.events import EventStore, ConcurrencyError, AssetCreated

        store = EventStore()
        store.append("asset-001", [AssetCreated(asset_id="001")])

        # Wrong expected version
        with pytest.raises(ConcurrencyError):
            store.append("asset-001", [AssetCreated()], expected_version=0)

    def test_stream_version(self):
        """Event store tracks stream version."""
        from tools.phoenix.events import EventStore, AssetCreated

        store = EventStore()

        assert store.get_stream_version("asset-001") == 0

        store.append("asset-001", [AssetCreated()])
        assert store.get_stream_version("asset-001") == 1

        store.append("asset-001", [AssetCreated(), AssetCreated()])
        assert store.get_stream_version("asset-001") == 3


class TestSaga:
    """Tests for saga pattern."""

    def test_saga_executes_steps(self):
        """Saga executes all steps in order."""
        from tools.phoenix.events import Saga, SagaState

        steps_executed = []

        saga = Saga("saga-001")
        saga.add_step("step1", lambda: steps_executed.append("step1"))
        saga.add_step("step2", lambda: steps_executed.append("step2"))
        saga.add_step("step3", lambda: steps_executed.append("step3"))

        success = saga.execute()

        assert success
        assert saga.state == SagaState.COMPLETED
        assert steps_executed == ["step1", "step2", "step3"]

    def test_saga_compensates_on_failure(self):
        """Saga compensates on failure."""
        from tools.phoenix.events import Saga, SagaState

        compensated = []

        def fail_step():
            raise ValueError("step failed")

        saga = Saga("saga-001")
        saga.add_step("step1", lambda: None, lambda: compensated.append("step1"))
        saga.add_step("step2", fail_step, lambda: compensated.append("step2"))

        success = saga.execute()

        assert not success
        assert saga.state == SagaState.FAILED
        assert "step1" in compensated


# ════════════════════════════════════════════════════════════════════════════
# CACHE TESTS
# ════════════════════════════════════════════════════════════════════════════


class TestLRUCache:
    """Tests for LRU cache."""

    def test_get_and_set(self):
        """LRU cache gets and sets values."""
        from tools.phoenix.cache import LRUCache

        cache = LRUCache(max_size=100)

        cache.set("key1", "value1")
        cache.set("key2", "value2")

        assert cache.get("key1") == "value1"
        assert cache.get("key2") == "value2"
        assert cache.get("nonexistent") is None

    def test_eviction_on_full(self):
        """LRU cache evicts least recently used."""
        from tools.phoenix.cache import LRUCache

        cache = LRUCache(max_size=2)

        cache.set("key1", "value1")
        cache.set("key2", "value2")

        # Access key1 to make it recently used
        cache.get("key1")

        # Add key3, should evict key2 (least recently used)
        cache.set("key3", "value3")

        assert cache.get("key1") == "value1"  # Still there
        assert cache.get("key2") is None       # Evicted
        assert cache.get("key3") == "value3"  # New

    def test_metrics_tracking(self):
        """LRU cache tracks metrics."""
        from tools.phoenix.cache import LRUCache

        cache = LRUCache(max_size=100)

        cache.set("key1", "value1")
        cache.get("key1")  # Hit
        cache.get("key2")  # Miss

        metrics = cache.metrics
        assert metrics.hits == 1
        assert metrics.misses == 1
        assert metrics.sets == 1


class TestTTLCache:
    """Tests for TTL cache."""

    def test_expiration(self):
        """TTL cache expires entries."""
        from tools.phoenix.cache import TTLCache

        cache = TTLCache(max_size=100, default_ttl_seconds=0.05)

        cache.set("key", "value")
        assert cache.get("key") == "value"

        time.sleep(0.1)
        assert cache.get("key") is None

    def test_custom_ttl(self):
        """TTL cache respects custom TTL per entry."""
        from tools.phoenix.cache import TTLCache

        cache = TTLCache(max_size=100, default_ttl_seconds=10)

        cache.set("short", "value", ttl=0.05)
        cache.set("long", "value", ttl=10)

        time.sleep(0.1)
        assert cache.get("short") is None
        assert cache.get("long") == "value"


class TestTieredCache:
    """Tests for tiered cache."""

    def test_tiered_lookup(self):
        """Tiered cache checks L1 before L2."""
        from tools.phoenix.cache import LRUCache, TieredCache

        l1 = LRUCache(max_size=10)
        l2 = LRUCache(max_size=100)

        tiered = TieredCache([l1, l2])

        # Set only in L2
        l2.set("key", "value")

        # Should find in L2 and promote to L1
        assert tiered.get("key") == "value"
        assert l1.get("key") == "value"  # Promoted


class TestCacheDecorator:
    """Tests for cache decorator."""

    def test_cached_decorator(self):
        """Cached decorator caches function results."""
        from tools.phoenix.cache import cached

        call_count = [0]

        @cached(max_size=100)
        def expensive_function(x):
            call_count[0] += 1
            return x * 2

        # First call computes
        assert expensive_function(5) == 10
        assert call_count[0] == 1

        # Second call uses cache
        assert expensive_function(5) == 10
        assert call_count[0] == 1  # Not incremented


class TestComputeCache:
    """Tests for compute cache."""

    def test_compute_on_miss(self):
        """Compute cache computes on miss."""
        from tools.phoenix.cache import ComputeCache

        call_count = [0]

        def compute(key):
            call_count[0] += 1
            return key.upper()

        cache = ComputeCache(compute=compute, max_size=100)

        # First call computes
        assert cache.get("hello") == "HELLO"
        assert call_count[0] == 1

        # Second call uses cache
        assert cache.get("hello") == "HELLO"
        assert call_count[0] == 1


# ════════════════════════════════════════════════════════════════════════════
# INTEGRATION TESTS
# ════════════════════════════════════════════════════════════════════════════


class TestInfrastructureIntegration:
    """Integration tests for infrastructure patterns."""

    def test_event_driven_caching(self):
        """Events can trigger cache invalidation."""
        from tools.phoenix.events import EventBus, AssetMigrated
        from tools.phoenix.cache import LRUCache

        cache = LRUCache(max_size=100)
        bus = EventBus()

        # Pre-populate cache
        cache.set("asset-001", {"jurisdiction": "uae-difc"})

        @bus.subscribe(AssetMigrated)
        def invalidate_cache(event):
            cache.delete(event.asset_id)

        # Migration should invalidate cache
        bus.publish(AssetMigrated(
            asset_id="asset-001",
            source_jurisdiction="uae-difc",
            target_jurisdiction="kz-aifc",
        ))

        assert cache.get("asset-001") is None

    def test_resilient_event_publishing(self):
        """Events can be published with resilience."""
        from tools.phoenix.events import EventBus, AssetCreated
        from tools.phoenix.resilience import CircuitBreaker

        bus = EventBus()
        breaker = CircuitBreaker("event-bus", failure_threshold=5)
        received = []

        @bus.subscribe(AssetCreated)
        def handler(event):
            received.append(event)

        @breaker
        def publish_event(event):
            bus.publish(event)

        publish_event(AssetCreated(asset_id="001"))
        assert len(received) == 1


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
