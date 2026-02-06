"""
PHOENIX Resilience Infrastructure

Production-grade fault tolerance patterns for the Smart Asset Operating System.
Implements circuit breaker, retry, bulkhead, and timeout patterns following
industry best practices from Netflix Hystrix and resilience4j.

Architecture
────────────

    ┌─────────────────────────────────────────────────────────────────────────┐
    │                       RESILIENCE PATTERNS                                │
    │                                                                          │
    │  Circuit Breaker     Retry Policy        Bulkhead           Timeout     │
    │  ├─ CLOSED state     ├─ Exponential      ├─ Semaphore       ├─ Hard    │
    │  ├─ OPEN state       ├─ Jitter           ├─ Thread pool     ├─ Soft    │
    │  ├─ HALF_OPEN        ├─ Max attempts     ├─ Queue depth     ├─ Cancel  │
    │  └─ Metrics          └─ Retryable exc    └─ Rejection       └─ Metrics │
    │                                                                          │
    │  Fallback            Rate Limiter        Hedging            Caching     │
    │  ├─ Default value    ├─ Token bucket     ├─ Parallel        ├─ TTL     │
    │  ├─ Degraded mode    ├─ Sliding window   ├─ Fastest wins    ├─ LRU     │
    │  └─ Circuit open     └─ Rejection        └─ Timeout         └─ Stale   │
    │                                                                          │
    └─────────────────────────────────────────────────────────────────────────┘

Design Principles
─────────────────

    Fail Fast: Circuit breakers prevent cascading failures by failing
    immediately when a downstream service is unavailable.

    Graceful Degradation: Fallbacks provide degraded functionality
    rather than complete failure.

    Resource Isolation: Bulkheads prevent a single slow dependency
    from consuming all resources.

    Bounded Latency: Timeouts ensure operations complete within
    acceptable time bounds.

Usage
─────

    from tools.phoenix.resilience import (
        CircuitBreaker,
        RetryPolicy,
        Bulkhead,
        Timeout,
        resilient,
    )

    # Declarative resilience with decorator
    @resilient(
        circuit_breaker=CircuitBreaker(failure_threshold=5),
        retry=RetryPolicy(max_attempts=3),
        timeout=Timeout(seconds=5.0),
    )
    def call_external_service():
        return external_api.call()

    # Programmatic resilience
    breaker = CircuitBreaker("anchor-service", failure_threshold=5)
    with breaker:
        result = anchor_service.submit_checkpoint()

Copyright (c) 2026 Momentum. All rights reserved.
"""

from __future__ import annotations

import functools
import random
import threading
import time
from abc import ABC, abstractmethod
from contextlib import contextmanager
from dataclasses import dataclass, field
from datetime import datetime, timezone
from enum import Enum, auto
from typing import (
    Any,
    Callable,
    Generic,
    Optional,
    TypeVar,
    Union,
    List,
    Dict,
    Set,
    Type,
)

T = TypeVar("T")
E = TypeVar("E", bound=Exception)


# ════════════════════════════════════════════════════════════════════════════
# CIRCUIT BREAKER
# ════════════════════════════════════════════════════════════════════════════


class CircuitState(Enum):
    """Circuit breaker states."""
    CLOSED = auto()      # Normal operation, requests pass through
    OPEN = auto()        # Circuit tripped, requests fail fast
    HALF_OPEN = auto()   # Testing if service recovered


@dataclass
class CircuitBreakerConfig:
    """Circuit breaker configuration."""
    failure_threshold: int = 5           # Failures before opening
    success_threshold: int = 3           # Successes to close from half-open
    timeout_seconds: float = 30.0        # Time before attempting recovery
    half_open_max_calls: int = 3         # Max calls in half-open state
    excluded_exceptions: tuple = ()      # Exceptions that don't count as failures
    included_exceptions: tuple = (Exception,)  # Exceptions that count as failures


@dataclass
class CircuitBreakerMetrics:
    """Circuit breaker metrics."""
    total_calls: int = 0
    successful_calls: int = 0
    failed_calls: int = 0
    rejected_calls: int = 0
    state_transitions: int = 0
    last_failure_time: Optional[datetime] = None
    last_success_time: Optional[datetime] = None
    consecutive_failures: int = 0
    consecutive_successes: int = 0


class CircuitBreakerError(Exception):
    """Raised when circuit breaker is open."""
    def __init__(self, breaker_name: str, state: CircuitState, message: str = ""):
        self.breaker_name = breaker_name
        self.state = state
        super().__init__(message or f"Circuit breaker '{breaker_name}' is {state.name}")


class CircuitBreaker:
    """
    Circuit breaker pattern implementation.

    Prevents cascading failures by failing fast when a service is unavailable.
    Transitions between CLOSED (normal), OPEN (failing fast), and HALF_OPEN
    (testing recovery) states.

    Example:
        breaker = CircuitBreaker("external-api", failure_threshold=5)

        @breaker
        def call_api():
            return api.call()

        # Or context manager
        with breaker:
            result = api.call()
    """

    def __init__(
        self,
        name: str,
        failure_threshold: int = 5,
        success_threshold: int = 3,
        timeout_seconds: float = 30.0,
        half_open_max_calls: int = 3,
        excluded_exceptions: tuple = (),
        on_state_change: Optional[Callable[[CircuitState, CircuitState], None]] = None,
    ):
        self.name = name
        self.config = CircuitBreakerConfig(
            failure_threshold=failure_threshold,
            success_threshold=success_threshold,
            timeout_seconds=timeout_seconds,
            half_open_max_calls=half_open_max_calls,
            excluded_exceptions=excluded_exceptions,
        )
        self._state = CircuitState.CLOSED
        self._metrics = CircuitBreakerMetrics()
        self._lock = threading.RLock()
        self._last_state_change = datetime.now(timezone.utc)
        self._half_open_calls = 0
        self._on_state_change = on_state_change

    @property
    def state(self) -> CircuitState:
        """Current circuit state."""
        with self._lock:
            self._check_state_timeout()
            return self._state

    @property
    def metrics(self) -> CircuitBreakerMetrics:
        """Current metrics."""
        with self._lock:
            return CircuitBreakerMetrics(
                total_calls=self._metrics.total_calls,
                successful_calls=self._metrics.successful_calls,
                failed_calls=self._metrics.failed_calls,
                rejected_calls=self._metrics.rejected_calls,
                state_transitions=self._metrics.state_transitions,
                last_failure_time=self._metrics.last_failure_time,
                last_success_time=self._metrics.last_success_time,
                consecutive_failures=self._metrics.consecutive_failures,
                consecutive_successes=self._metrics.consecutive_successes,
            )

    def _check_state_timeout(self) -> None:
        """Check if we should transition from OPEN to HALF_OPEN."""
        if self._state == CircuitState.OPEN:
            elapsed = (datetime.now(timezone.utc) - self._last_state_change).total_seconds()
            if elapsed >= self.config.timeout_seconds:
                self._transition_to(CircuitState.HALF_OPEN)

    def _transition_to(self, new_state: CircuitState) -> None:
        """Transition to a new state."""
        old_state = self._state
        if old_state != new_state:
            self._state = new_state
            self._last_state_change = datetime.now(timezone.utc)
            self._metrics.state_transitions += 1

            if new_state == CircuitState.HALF_OPEN:
                self._half_open_calls = 0
                self._metrics.consecutive_successes = 0
            elif new_state == CircuitState.CLOSED:
                self._metrics.consecutive_failures = 0

            if self._on_state_change:
                self._on_state_change(old_state, new_state)

    def _is_failure(self, exc: Exception) -> bool:
        """Check if exception should count as failure."""
        if isinstance(exc, self.config.excluded_exceptions):
            return False
        return isinstance(exc, self.config.included_exceptions)

    def _record_success(self) -> None:
        """Record successful call."""
        with self._lock:
            self._metrics.total_calls += 1
            self._metrics.successful_calls += 1
            self._metrics.last_success_time = datetime.now(timezone.utc)
            self._metrics.consecutive_successes += 1
            self._metrics.consecutive_failures = 0

            if self._state == CircuitState.HALF_OPEN:
                if self._metrics.consecutive_successes >= self.config.success_threshold:
                    self._transition_to(CircuitState.CLOSED)

    def _record_failure(self, exc: Exception) -> None:
        """Record failed call."""
        with self._lock:
            if not self._is_failure(exc):
                return

            self._metrics.total_calls += 1
            self._metrics.failed_calls += 1
            self._metrics.last_failure_time = datetime.now(timezone.utc)
            self._metrics.consecutive_failures += 1
            self._metrics.consecutive_successes = 0

            if self._state == CircuitState.CLOSED:
                if self._metrics.consecutive_failures >= self.config.failure_threshold:
                    self._transition_to(CircuitState.OPEN)
            elif self._state == CircuitState.HALF_OPEN:
                self._transition_to(CircuitState.OPEN)

    def _acquire(self) -> bool:
        """Acquire permission to make a call."""
        with self._lock:
            self._check_state_timeout()

            if self._state == CircuitState.CLOSED:
                return True
            elif self._state == CircuitState.OPEN:
                self._metrics.rejected_calls += 1
                return False
            elif self._state == CircuitState.HALF_OPEN:
                if self._half_open_calls < self.config.half_open_max_calls:
                    self._half_open_calls += 1
                    return True
                self._metrics.rejected_calls += 1
                return False
        return False

    def __enter__(self):
        """Context manager entry."""
        if not self._acquire():
            raise CircuitBreakerError(self.name, self._state)
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        """Context manager exit."""
        if exc_type is not None:
            self._record_failure(exc_val)
        else:
            self._record_success()
        return False

    def __call__(self, func: Callable[..., T]) -> Callable[..., T]:
        """Decorator for circuit breaker protection."""
        @functools.wraps(func)
        def wrapper(*args, **kwargs) -> T:
            if not self._acquire():
                raise CircuitBreakerError(self.name, self._state)
            try:
                result = func(*args, **kwargs)
                self._record_success()
                return result
            except Exception as e:
                self._record_failure(e)
                raise
        return wrapper

    def reset(self) -> None:
        """Reset circuit breaker to closed state."""
        with self._lock:
            self._transition_to(CircuitState.CLOSED)
            self._metrics = CircuitBreakerMetrics()


# ════════════════════════════════════════════════════════════════════════════
# RETRY POLICY
# ════════════════════════════════════════════════════════════════════════════


class BackoffStrategy(Enum):
    """Retry backoff strategies."""
    FIXED = auto()           # Fixed delay between retries
    EXPONENTIAL = auto()     # Exponential backoff (2^n)
    EXPONENTIAL_JITTER = auto()  # Exponential with random jitter
    LINEAR = auto()          # Linear increase


@dataclass
class RetryConfig:
    """Retry policy configuration."""
    max_attempts: int = 3
    base_delay_seconds: float = 1.0
    max_delay_seconds: float = 60.0
    backoff_strategy: BackoffStrategy = BackoffStrategy.EXPONENTIAL_JITTER
    jitter_factor: float = 0.5  # Random factor 0-1
    retryable_exceptions: tuple = (Exception,)
    non_retryable_exceptions: tuple = ()


@dataclass
class RetryMetrics:
    """Retry metrics."""
    total_attempts: int = 0
    successful_attempts: int = 0
    failed_attempts: int = 0
    retries_exhausted: int = 0
    total_retry_delay_seconds: float = 0.0


class RetryExhaustedError(Exception):
    """Raised when all retry attempts are exhausted."""
    def __init__(self, attempts: int, last_exception: Exception):
        self.attempts = attempts
        self.last_exception = last_exception
        super().__init__(f"Retry exhausted after {attempts} attempts: {last_exception}")


class RetryPolicy:
    """
    Retry policy with configurable backoff strategies.

    Supports exponential backoff with jitter for distributed systems,
    preventing thundering herd problems.

    Example:
        retry = RetryPolicy(max_attempts=3, backoff=BackoffStrategy.EXPONENTIAL_JITTER)

        @retry
        def flaky_operation():
            return external_service.call()

        # Or programmatic
        result = retry.execute(lambda: external_service.call())
    """

    def __init__(
        self,
        max_attempts: int = 3,
        base_delay_seconds: float = 1.0,
        max_delay_seconds: float = 60.0,
        backoff_strategy: BackoffStrategy = BackoffStrategy.EXPONENTIAL_JITTER,
        jitter_factor: float = 0.5,
        retryable_exceptions: tuple = (Exception,),
        non_retryable_exceptions: tuple = (),
        on_retry: Optional[Callable[[int, Exception, float], None]] = None,
    ):
        self.config = RetryConfig(
            max_attempts=max_attempts,
            base_delay_seconds=base_delay_seconds,
            max_delay_seconds=max_delay_seconds,
            backoff_strategy=backoff_strategy,
            jitter_factor=jitter_factor,
            retryable_exceptions=retryable_exceptions,
            non_retryable_exceptions=non_retryable_exceptions,
        )
        self._metrics = RetryMetrics()
        self._lock = threading.Lock()
        self._on_retry = on_retry

    @property
    def metrics(self) -> RetryMetrics:
        """Current retry metrics."""
        with self._lock:
            return RetryMetrics(
                total_attempts=self._metrics.total_attempts,
                successful_attempts=self._metrics.successful_attempts,
                failed_attempts=self._metrics.failed_attempts,
                retries_exhausted=self._metrics.retries_exhausted,
                total_retry_delay_seconds=self._metrics.total_retry_delay_seconds,
            )

    def _calculate_delay(self, attempt: int) -> float:
        """Calculate delay before next retry."""
        base = self.config.base_delay_seconds

        if self.config.backoff_strategy == BackoffStrategy.FIXED:
            delay = base
        elif self.config.backoff_strategy == BackoffStrategy.LINEAR:
            delay = base * attempt
        elif self.config.backoff_strategy == BackoffStrategy.EXPONENTIAL:
            delay = base * (2 ** (attempt - 1))
        elif self.config.backoff_strategy == BackoffStrategy.EXPONENTIAL_JITTER:
            exp_delay = base * (2 ** (attempt - 1))
            jitter = random.uniform(0, self.config.jitter_factor * exp_delay)
            delay = exp_delay + jitter
        else:
            delay = base

        return min(delay, self.config.max_delay_seconds)

    def _is_retryable(self, exc: Exception) -> bool:
        """Check if exception is retryable."""
        if isinstance(exc, self.config.non_retryable_exceptions):
            return False
        return isinstance(exc, self.config.retryable_exceptions)

    def execute(self, func: Callable[[], T]) -> T:
        """Execute function with retry policy."""
        last_exception: Optional[Exception] = None

        for attempt in range(1, self.config.max_attempts + 1):
            with self._lock:
                self._metrics.total_attempts += 1

            try:
                result = func()
                with self._lock:
                    self._metrics.successful_attempts += 1
                return result
            except Exception as e:
                last_exception = e
                with self._lock:
                    self._metrics.failed_attempts += 1

                if not self._is_retryable(e):
                    raise

                if attempt < self.config.max_attempts:
                    delay = self._calculate_delay(attempt)
                    with self._lock:
                        self._metrics.total_retry_delay_seconds += delay

                    if self._on_retry:
                        self._on_retry(attempt, e, delay)

                    time.sleep(delay)

        with self._lock:
            self._metrics.retries_exhausted += 1

        raise RetryExhaustedError(self.config.max_attempts, last_exception)

    def __call__(self, func: Callable[..., T]) -> Callable[..., T]:
        """Decorator for retry protection."""
        @functools.wraps(func)
        def wrapper(*args, **kwargs) -> T:
            return self.execute(lambda: func(*args, **kwargs))
        return wrapper


# ════════════════════════════════════════════════════════════════════════════
# BULKHEAD
# ════════════════════════════════════════════════════════════════════════════


class BulkheadFullError(Exception):
    """Raised when bulkhead is at capacity."""
    def __init__(self, name: str, max_concurrent: int):
        self.name = name
        self.max_concurrent = max_concurrent
        super().__init__(f"Bulkhead '{name}' at capacity ({max_concurrent})")


@dataclass
class BulkheadMetrics:
    """Bulkhead metrics."""
    total_calls: int = 0
    successful_calls: int = 0
    rejected_calls: int = 0
    current_concurrent: int = 0
    max_concurrent_reached: int = 0


class Bulkhead:
    """
    Bulkhead pattern for resource isolation.

    Limits concurrent executions to prevent a single slow dependency
    from consuming all resources and causing cascading failures.

    Example:
        bulkhead = Bulkhead("database", max_concurrent=10)

        @bulkhead
        def query_database():
            return db.execute(query)

        # Or context manager
        with bulkhead:
            result = db.execute(query)
    """

    def __init__(
        self,
        name: str,
        max_concurrent: int = 10,
        max_wait_seconds: float = 0.0,  # 0 = no waiting
        on_rejection: Optional[Callable[[], None]] = None,
    ):
        self.name = name
        self.max_concurrent = max_concurrent
        self.max_wait_seconds = max_wait_seconds
        self._semaphore = threading.Semaphore(max_concurrent)
        self._metrics = BulkheadMetrics()
        self._lock = threading.Lock()
        self._on_rejection = on_rejection

    @property
    def metrics(self) -> BulkheadMetrics:
        """Current bulkhead metrics."""
        with self._lock:
            return BulkheadMetrics(
                total_calls=self._metrics.total_calls,
                successful_calls=self._metrics.successful_calls,
                rejected_calls=self._metrics.rejected_calls,
                current_concurrent=self._metrics.current_concurrent,
                max_concurrent_reached=self._metrics.max_concurrent_reached,
            )

    @property
    def available_permits(self) -> int:
        """Number of available permits."""
        with self._lock:
            return self.max_concurrent - self._metrics.current_concurrent

    def _acquire(self) -> bool:
        """Acquire a permit."""
        timeout = self.max_wait_seconds if self.max_wait_seconds > 0 else None
        acquired = self._semaphore.acquire(blocking=timeout is not None, timeout=timeout)

        with self._lock:
            self._metrics.total_calls += 1
            if acquired:
                self._metrics.current_concurrent += 1
                self._metrics.max_concurrent_reached = max(
                    self._metrics.max_concurrent_reached,
                    self._metrics.current_concurrent
                )
            else:
                self._metrics.rejected_calls += 1
                if self._on_rejection:
                    self._on_rejection()

        return acquired

    def _release(self) -> None:
        """Release a permit."""
        self._semaphore.release()
        with self._lock:
            self._metrics.current_concurrent -= 1

    def __enter__(self):
        """Context manager entry."""
        if not self._acquire():
            raise BulkheadFullError(self.name, self.max_concurrent)
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        """Context manager exit."""
        self._release()
        with self._lock:
            self._metrics.successful_calls += 1
        return False

    def __call__(self, func: Callable[..., T]) -> Callable[..., T]:
        """Decorator for bulkhead protection."""
        @functools.wraps(func)
        def wrapper(*args, **kwargs) -> T:
            if not self._acquire():
                raise BulkheadFullError(self.name, self.max_concurrent)
            try:
                result = func(*args, **kwargs)
                with self._lock:
                    self._metrics.successful_calls += 1
                return result
            finally:
                self._release()
        return wrapper


# ════════════════════════════════════════════════════════════════════════════
# TIMEOUT
# ════════════════════════════════════════════════════════════════════════════


class TimeoutError(Exception):
    """Raised when operation exceeds timeout."""
    def __init__(self, operation: str, timeout_seconds: float):
        self.operation = operation
        self.timeout_seconds = timeout_seconds
        super().__init__(f"Operation '{operation}' timed out after {timeout_seconds}s")


@dataclass
class TimeoutMetrics:
    """Timeout metrics."""
    total_calls: int = 0
    successful_calls: int = 0
    timed_out_calls: int = 0
    total_duration_seconds: float = 0.0


class Timeout:
    """
    Timeout pattern for bounded latency.

    Ensures operations complete within acceptable time bounds.
    Uses threading for true timeout (not just cooperative).

    Example:
        timeout = Timeout(seconds=5.0)

        @timeout
        def slow_operation():
            return external_service.call()

        # Or programmatic
        result = timeout.execute(lambda: external_service.call())

    Note:
        For true interruption, consider using concurrent.futures.
        This implementation provides cooperative timeout.
    """

    def __init__(
        self,
        seconds: float,
        name: str = "operation",
        on_timeout: Optional[Callable[[], None]] = None,
    ):
        self.seconds = seconds
        self.name = name
        self._metrics = TimeoutMetrics()
        self._lock = threading.Lock()
        self._on_timeout = on_timeout

    @property
    def metrics(self) -> TimeoutMetrics:
        """Current timeout metrics."""
        with self._lock:
            return TimeoutMetrics(
                total_calls=self._metrics.total_calls,
                successful_calls=self._metrics.successful_calls,
                timed_out_calls=self._metrics.timed_out_calls,
                total_duration_seconds=self._metrics.total_duration_seconds,
            )

    def execute(self, func: Callable[[], T]) -> T:
        """Execute function with timeout."""
        import concurrent.futures

        with self._lock:
            self._metrics.total_calls += 1

        start_time = time.monotonic()

        with concurrent.futures.ThreadPoolExecutor(max_workers=1) as executor:
            future = executor.submit(func)
            try:
                result = future.result(timeout=self.seconds)
                duration = time.monotonic() - start_time
                with self._lock:
                    self._metrics.successful_calls += 1
                    self._metrics.total_duration_seconds += duration
                return result
            except concurrent.futures.TimeoutError:
                with self._lock:
                    self._metrics.timed_out_calls += 1
                if self._on_timeout:
                    self._on_timeout()
                raise TimeoutError(self.name, self.seconds)

    def __call__(self, func: Callable[..., T]) -> Callable[..., T]:
        """Decorator for timeout protection."""
        @functools.wraps(func)
        def wrapper(*args, **kwargs) -> T:
            return self.execute(lambda: func(*args, **kwargs))
        return wrapper


# ════════════════════════════════════════════════════════════════════════════
# FALLBACK
# ════════════════════════════════════════════════════════════════════════════


class Fallback(Generic[T]):
    """
    Fallback pattern for graceful degradation.

    Provides alternative behavior when primary operation fails.
    Can be combined with circuit breaker for circuit-open fallback.

    Example:
        fallback = Fallback(
            fallback_value={"status": "degraded"},
            fallback_func=lambda: cache.get_stale(),
        )

        @fallback
        def get_data():
            return api.fetch_data()
    """

    def __init__(
        self,
        fallback_value: Optional[T] = None,
        fallback_func: Optional[Callable[[], T]] = None,
        exceptions: tuple = (Exception,),
        on_fallback: Optional[Callable[[Exception], None]] = None,
    ):
        self.fallback_value = fallback_value
        self.fallback_func = fallback_func
        self.exceptions = exceptions
        self._on_fallback = on_fallback
        self._fallback_count = 0
        self._lock = threading.Lock()

    @property
    def fallback_count(self) -> int:
        """Number of times fallback was used."""
        with self._lock:
            return self._fallback_count

    def execute(self, func: Callable[[], T]) -> T:
        """Execute with fallback."""
        try:
            return func()
        except self.exceptions as e:
            with self._lock:
                self._fallback_count += 1

            if self._on_fallback:
                self._on_fallback(e)

            if self.fallback_func:
                return self.fallback_func()
            return self.fallback_value

    def __call__(self, func: Callable[..., T]) -> Callable[..., T]:
        """Decorator for fallback protection."""
        @functools.wraps(func)
        def wrapper(*args, **kwargs) -> T:
            return self.execute(lambda: func(*args, **kwargs))
        return wrapper


# ════════════════════════════════════════════════════════════════════════════
# COMPOSITE RESILIENCE
# ════════════════════════════════════════════════════════════════════════════


@dataclass
class ResilienceConfig:
    """Combined resilience configuration."""
    circuit_breaker: Optional[CircuitBreaker] = None
    retry: Optional[RetryPolicy] = None
    bulkhead: Optional[Bulkhead] = None
    timeout: Optional[Timeout] = None
    fallback: Optional[Fallback] = None


def resilient(
    circuit_breaker: Optional[CircuitBreaker] = None,
    retry: Optional[RetryPolicy] = None,
    bulkhead: Optional[Bulkhead] = None,
    timeout: Optional[Timeout] = None,
    fallback: Optional[Fallback] = None,
) -> Callable[[Callable[..., T]], Callable[..., T]]:
    """
    Composite resilience decorator.

    Applies multiple resilience patterns in the correct order:
    1. Fallback (outermost - catches everything)
    2. Circuit Breaker (fail fast if open)
    3. Bulkhead (limit concurrency)
    4. Timeout (bound latency)
    5. Retry (retry transient failures)

    Example:
        @resilient(
            circuit_breaker=CircuitBreaker("api", failure_threshold=5),
            retry=RetryPolicy(max_attempts=3),
            timeout=Timeout(seconds=5.0),
            fallback=Fallback(fallback_value={"cached": True}),
        )
        def call_api():
            return api.call()
    """
    def decorator(func: Callable[..., T]) -> Callable[..., T]:
        @functools.wraps(func)
        def wrapper(*args, **kwargs) -> T:
            def execute():
                # Innermost: actual function
                return func(*args, **kwargs)

            call = execute

            # Apply in reverse order (innermost first)
            if retry:
                inner = call
                call = lambda: retry.execute(inner)

            if timeout:
                inner = call
                call = lambda: timeout.execute(inner)

            if bulkhead:
                inner = call
                def bulkhead_call():
                    with bulkhead:
                        return inner()
                call = bulkhead_call

            if circuit_breaker:
                inner = call
                def breaker_call():
                    with circuit_breaker:
                        return inner()
                call = breaker_call

            if fallback:
                inner = call
                call = lambda: fallback.execute(inner)

            return call()

        return wrapper
    return decorator


# ════════════════════════════════════════════════════════════════════════════
# RESILIENCE REGISTRY
# ════════════════════════════════════════════════════════════════════════════


class ResilienceRegistry:
    """
    Registry for managing resilience components.

    Provides centralized management of circuit breakers, bulkheads,
    and other resilience components with metrics aggregation.

    Example:
        registry = ResilienceRegistry()

        breaker = registry.circuit_breaker("api-service", failure_threshold=5)
        bulkhead = registry.bulkhead("database", max_concurrent=10)

        # Get aggregated metrics
        metrics = registry.get_all_metrics()
    """

    _instance: Optional['ResilienceRegistry'] = None
    _lock = threading.Lock()

    def __init__(self):
        self._circuit_breakers: Dict[str, CircuitBreaker] = {}
        self._bulkheads: Dict[str, Bulkhead] = {}
        self._retry_policies: Dict[str, RetryPolicy] = {}
        self._timeouts: Dict[str, Timeout] = {}
        self._lock = threading.RLock()

    @classmethod
    def get_instance(cls) -> 'ResilienceRegistry':
        """Get singleton instance."""
        if cls._instance is None:
            with cls._lock:
                if cls._instance is None:
                    cls._instance = cls()
        return cls._instance

    def circuit_breaker(
        self,
        name: str,
        failure_threshold: int = 5,
        **kwargs,
    ) -> CircuitBreaker:
        """Get or create a circuit breaker."""
        with self._lock:
            if name not in self._circuit_breakers:
                self._circuit_breakers[name] = CircuitBreaker(
                    name, failure_threshold=failure_threshold, **kwargs
                )
            return self._circuit_breakers[name]

    def bulkhead(
        self,
        name: str,
        max_concurrent: int = 10,
        **kwargs,
    ) -> Bulkhead:
        """Get or create a bulkhead."""
        with self._lock:
            if name not in self._bulkheads:
                self._bulkheads[name] = Bulkhead(
                    name, max_concurrent=max_concurrent, **kwargs
                )
            return self._bulkheads[name]

    def retry_policy(
        self,
        name: str,
        max_attempts: int = 3,
        **kwargs,
    ) -> RetryPolicy:
        """Get or create a retry policy."""
        with self._lock:
            if name not in self._retry_policies:
                self._retry_policies[name] = RetryPolicy(
                    max_attempts=max_attempts, **kwargs
                )
            return self._retry_policies[name]

    def timeout(
        self,
        name: str,
        seconds: float = 30.0,
    ) -> Timeout:
        """Get or create a timeout."""
        with self._lock:
            if name not in self._timeouts:
                self._timeouts[name] = Timeout(seconds, name=name)
            return self._timeouts[name]

    def get_all_metrics(self) -> Dict[str, Any]:
        """Get aggregated metrics from all components."""
        with self._lock:
            return {
                "circuit_breakers": {
                    name: {
                        "state": cb.state.name,
                        "metrics": cb.metrics.__dict__,
                    }
                    for name, cb in self._circuit_breakers.items()
                },
                "bulkheads": {
                    name: {
                        "available_permits": bh.available_permits,
                        "metrics": bh.metrics.__dict__,
                    }
                    for name, bh in self._bulkheads.items()
                },
                "retry_policies": {
                    name: rp.metrics.__dict__
                    for name, rp in self._retry_policies.items()
                },
                "timeouts": {
                    name: to.metrics.__dict__
                    for name, to in self._timeouts.items()
                },
            }

    def reset_all(self) -> None:
        """Reset all circuit breakers."""
        with self._lock:
            for cb in self._circuit_breakers.values():
                cb.reset()


# ════════════════════════════════════════════════════════════════════════════
# CONVENIENCE FUNCTIONS
# ════════════════════════════════════════════════════════════════════════════


def get_resilience_registry() -> ResilienceRegistry:
    """Get the global resilience registry."""
    return ResilienceRegistry.get_instance()


# ════════════════════════════════════════════════════════════════════════════
# MODULE EXPORTS
# ════════════════════════════════════════════════════════════════════════════


__all__ = [
    # Circuit Breaker
    "CircuitState",
    "CircuitBreakerConfig",
    "CircuitBreakerMetrics",
    "CircuitBreakerError",
    "CircuitBreaker",
    # Retry
    "BackoffStrategy",
    "RetryConfig",
    "RetryMetrics",
    "RetryExhaustedError",
    "RetryPolicy",
    # Bulkhead
    "BulkheadFullError",
    "BulkheadMetrics",
    "Bulkhead",
    # Timeout
    "TimeoutError",
    "TimeoutMetrics",
    "Timeout",
    # Fallback
    "Fallback",
    # Composite
    "ResilienceConfig",
    "resilient",
    # Registry
    "ResilienceRegistry",
    "get_resilience_registry",
]
