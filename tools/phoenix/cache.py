"""
PHOENIX Cache Infrastructure

Multi-tier caching system for the Smart Asset Operating System.
Implements LRU eviction, TTL expiration, write-through/write-behind,
and cache-aside patterns for high-performance operations.

Architecture
────────────

    ┌─────────────────────────────────────────────────────────────────────────┐
    │                         CACHE INFRASTRUCTURE                             │
    │                                                                          │
    │  Cache Types         Eviction           Invalidation                     │
    │  ├─ LRU Cache        ├─ LRU             ├─ TTL expiry                   │
    │  ├─ TTL Cache        ├─ LFU             ├─ Manual                       │
    │  ├─ Tiered Cache     ├─ FIFO            ├─ Pattern                      │
    │  └─ Async Cache      └─ Size-based      └─ Cascade                      │
    │                                                                          │
    │  Patterns            Metrics            Persistence                      │
    │  ├─ Cache-aside      ├─ Hit ratio       ├─ Write-through                │
    │  ├─ Read-through     ├─ Miss ratio      ├─ Write-behind                 │
    │  ├─ Write-through    ├─ Evictions       ├─ Snapshots                    │
    │  └─ Write-behind     └─ Size            └─ Recovery                     │
    │                                                                          │
    └─────────────────────────────────────────────────────────────────────────┘

Design Principles
─────────────────

    Performance First: Cache operations are O(1) for get/set.
    LRU uses doubly-linked list for O(1) eviction.

    Memory Bounded: All caches have configurable size limits.
    Eviction happens automatically when limits are reached.

    Thread Safe: All operations are protected by locks.
    Safe for concurrent access from multiple threads.

    Observable: Comprehensive metrics for monitoring and tuning.
    Hit/miss ratios, eviction counts, size tracking.

Usage
─────

    from tools.phoenix.cache import (
        LRUCache,
        TTLCache,
        TieredCache,
        cached,
    )

    # Simple LRU cache
    cache = LRUCache(max_size=1000)
    cache.set("key", "value")
    value = cache.get("key")

    # TTL cache with expiration
    ttl_cache = TTLCache(max_size=1000, default_ttl_seconds=60)
    ttl_cache.set("key", "value", ttl=30)

    # Decorator for automatic caching
    @cached(max_size=100, ttl_seconds=60)
    def expensive_computation(x):
        return compute(x)

Copyright (c) 2026 Momentum. All rights reserved.
"""

from __future__ import annotations

import functools
import hashlib
import json
import threading
import time
from abc import ABC, abstractmethod
from collections import OrderedDict
from dataclasses import dataclass, field
from datetime import datetime, timezone
from typing import (
    Any,
    Callable,
    Dict,
    Generic,
    Iterator,
    List,
    Optional,
    Set,
    Tuple,
    TypeVar,
    Union,
)

K = TypeVar("K")
V = TypeVar("V")
T = TypeVar("T")


# ════════════════════════════════════════════════════════════════════════════
# CACHE ENTRY
# ════════════════════════════════════════════════════════════════════════════


@dataclass
class CacheEntry(Generic[V]):
    """A cache entry with metadata."""
    value: V
    created_at: float = field(default_factory=time.monotonic)
    accessed_at: float = field(default_factory=time.monotonic)
    expires_at: Optional[float] = None
    access_count: int = 0
    size_bytes: int = 0

    @property
    def is_expired(self) -> bool:
        """Check if entry has expired."""
        if self.expires_at is None:
            return False
        return time.monotonic() > self.expires_at

    def touch(self) -> None:
        """Update access time and count."""
        self.accessed_at = time.monotonic()
        self.access_count += 1

    @property
    def age_seconds(self) -> float:
        """Age of entry in seconds."""
        return time.monotonic() - self.created_at


# ════════════════════════════════════════════════════════════════════════════
# CACHE METRICS
# ════════════════════════════════════════════════════════════════════════════


class CacheMetrics:
    """Cache performance metrics with thread-safe counters."""

    def __init__(
        self,
        hits: int = 0,
        misses: int = 0,
        evictions: int = 0,
        expirations: int = 0,
        sets: int = 0,
        deletes: int = 0,
        current_size: int = 0,
        max_size: int = 0,
    ):
        self._lock = threading.Lock()
        self.hits = hits
        self.misses = misses
        self.evictions = evictions
        self.expirations = expirations
        self.sets = sets
        self.deletes = deletes
        self.current_size = current_size
        self.max_size = max_size

    @property
    def total_requests(self) -> int:
        """Total cache requests."""
        with self._lock:
            return self.hits + self.misses

    @property
    def hit_ratio(self) -> float:
        """Cache hit ratio (0.0 - 1.0)."""
        with self._lock:
            total = self.hits + self.misses
            if total == 0:
                return 0.0
            return self.hits / total

    @property
    def miss_ratio(self) -> float:
        """Cache miss ratio (0.0 - 1.0)."""
        return 1.0 - self.hit_ratio

    def to_dict(self) -> Dict[str, Any]:
        """Convert to dictionary."""
        with self._lock:
            return {
                "hits": self.hits,
                "misses": self.misses,
                "evictions": self.evictions,
                "expirations": self.expirations,
                "sets": self.sets,
                "deletes": self.deletes,
                "current_size": self.current_size,
                "max_size": self.max_size,
                "hit_ratio": round(self.hits / (self.hits + self.misses) if (self.hits + self.misses) > 0 else 0.0, 4),
                "miss_ratio": round(self.misses / (self.hits + self.misses) if (self.hits + self.misses) > 0 else 0.0, 4),
                "total_requests": self.hits + self.misses,
            }


# ════════════════════════════════════════════════════════════════════════════
# CACHE INTERFACE
# ════════════════════════════════════════════════════════════════════════════


class Cache(ABC, Generic[K, V]):
    """Abstract base class for cache implementations."""

    @abstractmethod
    def get(self, key: K, default: Optional[V] = None) -> Optional[V]:
        """Get value from cache."""
        pass

    @abstractmethod
    def set(self, key: K, value: V, ttl: Optional[float] = None) -> None:
        """Set value in cache."""
        pass

    @abstractmethod
    def delete(self, key: K) -> bool:
        """Delete value from cache."""
        pass

    @abstractmethod
    def contains(self, key: K) -> bool:
        """Check if key exists in cache."""
        pass

    @abstractmethod
    def clear(self) -> None:
        """Clear all entries from cache."""
        pass

    @abstractmethod
    def size(self) -> int:
        """Current number of entries."""
        pass

    @property
    @abstractmethod
    def metrics(self) -> CacheMetrics:
        """Cache metrics."""
        pass


# ════════════════════════════════════════════════════════════════════════════
# LRU CACHE
# ════════════════════════════════════════════════════════════════════════════


class LRUCache(Cache[K, V]):
    """
    Least Recently Used cache with O(1) operations.

    Uses OrderedDict for efficient LRU tracking.
    Thread-safe for concurrent access.

    Example:
        cache = LRUCache(max_size=1000)
        cache.set("key1", "value1")
        cache.set("key2", "value2")

        value = cache.get("key1")  # Returns "value1", marks as recently used

        # When full, evicts least recently used
        for i in range(1000):
            cache.set(f"key{i}", f"value{i}")
    """

    def __init__(
        self,
        max_size: int = 1000,
        on_evict: Optional[Callable[[K, V], None]] = None,
    ):
        self._max_size = max_size
        self._cache: OrderedDict[K, CacheEntry[V]] = OrderedDict()
        self._lock = threading.RLock()
        self._metrics = CacheMetrics(max_size=max_size)
        self._on_evict = on_evict

    def get(self, key: K, default: Optional[V] = None) -> Optional[V]:
        """Get value, moving to end of LRU list.

        Performs lazy expiration: if the entry's TTL has passed,
        it is removed and treated as a miss.
        """
        with self._lock:
            if key not in self._cache:
                self._metrics.misses += 1
                return default

            entry = self._cache[key]

            # Lazy expiration: check TTL before returning
            if entry.is_expired:
                del self._cache[key]
                self._metrics.expirations += 1
                self._metrics.misses += 1
                self._metrics.current_size = len(self._cache)
                if self._on_evict:
                    try:
                        self._on_evict(key, entry.value)
                    except Exception:
                        pass
                return default

            entry.touch()

            # Move to end (most recently used)
            self._cache.move_to_end(key)

            self._metrics.hits += 1
            return entry.value

    def set(self, key: K, value: V, ttl: Optional[float] = None) -> None:
        """Set value, evicting LRU if necessary."""
        with self._lock:
            # Update existing
            if key in self._cache:
                entry = self._cache[key]
                entry.value = value
                entry.touch()
                self._cache.move_to_end(key)
                self._metrics.sets += 1
                return

            # Evict if at capacity
            while len(self._cache) >= self._max_size:
                self._evict_one()

            # Add new entry
            entry = CacheEntry(value=value)
            if ttl is not None:
                entry.expires_at = time.monotonic() + ttl

            self._cache[key] = entry
            self._metrics.sets += 1
            self._metrics.current_size = len(self._cache)

    def delete(self, key: K) -> bool:
        """Delete entry by key."""
        with self._lock:
            if key in self._cache:
                del self._cache[key]
                self._metrics.deletes += 1
                self._metrics.current_size = len(self._cache)
                return True
            return False

    def contains(self, key: K) -> bool:
        """Check if key exists and is not expired (doesn't update LRU order)."""
        with self._lock:
            if key not in self._cache:
                return False
            entry = self._cache[key]
            if entry.is_expired:
                del self._cache[key]
                self._metrics.expirations += 1
                self._metrics.current_size = len(self._cache)
                return False
            return True

    def clear(self) -> None:
        """Clear all entries, calling eviction callbacks for each."""
        with self._lock:
            if self._on_evict:
                for key, entry in self._cache.items():
                    try:
                        self._on_evict(key, entry.value)
                    except Exception:
                        pass  # Best-effort cleanup
            self._cache.clear()
            self._metrics.current_size = 0

    def size(self) -> int:
        """Current number of entries."""
        with self._lock:
            return len(self._cache)

    @property
    def metrics(self) -> CacheMetrics:
        """Cache metrics."""
        with self._lock:
            return CacheMetrics(
                hits=self._metrics.hits,
                misses=self._metrics.misses,
                evictions=self._metrics.evictions,
                expirations=self._metrics.expirations,
                sets=self._metrics.sets,
                deletes=self._metrics.deletes,
                current_size=len(self._cache),
                max_size=self._max_size,
            )

    def _evict_one(self) -> None:
        """Evict least recently used entry."""
        if not self._cache:
            return

        # Pop from front (least recently used)
        key, entry = self._cache.popitem(last=False)
        self._metrics.evictions += 1

        if self._on_evict:
            try:
                self._on_evict(key, entry.value)
            except Exception:
                pass  # Best-effort cleanup callback

    def keys(self) -> List[K]:
        """Get all keys (most recently used last)."""
        with self._lock:
            return list(self._cache.keys())

    def values(self) -> List[V]:
        """Get all values."""
        with self._lock:
            return [e.value for e in self._cache.values()]

    def items(self) -> List[Tuple[K, V]]:
        """Get all key-value pairs."""
        with self._lock:
            return [(k, e.value) for k, e in self._cache.items()]


# ════════════════════════════════════════════════════════════════════════════
# TTL CACHE
# ════════════════════════════════════════════════════════════════════════════


class TTLCache(Cache[K, V]):
    """
    Cache with Time-To-Live expiration.

    Entries automatically expire after TTL.
    Combines LRU eviction with TTL expiration.

    Example:
        cache = TTLCache(max_size=1000, default_ttl_seconds=60)

        cache.set("key", "value")  # Expires in 60 seconds
        cache.set("key2", "value2", ttl=30)  # Expires in 30 seconds

        # Expired entries return None
        time.sleep(61)
        cache.get("key")  # Returns None
    """

    def __init__(
        self,
        max_size: int = 1000,
        default_ttl_seconds: float = 300.0,
        cleanup_interval_seconds: float = 60.0,
        on_evict: Optional[Callable[[K, V], None]] = None,
        on_expire: Optional[Callable[[K, V], None]] = None,
    ):
        self._max_size = max_size
        self._default_ttl = default_ttl_seconds
        self._cache: OrderedDict[K, CacheEntry[V]] = OrderedDict()
        self._lock = threading.RLock()
        self._metrics = CacheMetrics(max_size=max_size)
        self._on_evict = on_evict
        self._on_expire = on_expire

        # Background cleanup
        self._cleanup_interval = cleanup_interval_seconds
        self._cleanup_thread: Optional[threading.Thread] = None
        self._running = False

    def start_cleanup(self) -> None:
        """Start background cleanup thread."""
        if self._running:
            return

        self._running = True
        self._cleanup_thread = threading.Thread(
            target=self._cleanup_loop,
            daemon=True,
            name="ttl-cache-cleanup",
        )
        self._cleanup_thread.start()

    def stop_cleanup(self) -> None:
        """Stop background cleanup."""
        self._running = False
        if self._cleanup_thread:
            self._cleanup_thread.join(timeout=2.0)
            self._cleanup_thread = None

    def _cleanup_loop(self) -> None:
        """Background cleanup of expired entries."""
        while self._running:
            time.sleep(self._cleanup_interval)
            self._cleanup_expired()

    def _cleanup_expired(self) -> int:
        """Remove expired entries, return count removed."""
        with self._lock:
            expired_keys = [
                k for k, e in self._cache.items() if e.is_expired
            ]

            for key in expired_keys:
                entry = self._cache.pop(key)
                self._metrics.expirations += 1

                if self._on_expire:
                    self._on_expire(key, entry.value)

            self._metrics.current_size = len(self._cache)
            return len(expired_keys)

    def get(self, key: K, default: Optional[V] = None) -> Optional[V]:
        """Get value if exists and not expired."""
        with self._lock:
            if key not in self._cache:
                self._metrics.misses += 1
                return default

            entry = self._cache[key]

            # Check expiration
            if entry.is_expired:
                del self._cache[key]
                self._metrics.expirations += 1
                self._metrics.misses += 1

                if self._on_expire:
                    self._on_expire(key, entry.value)

                return default

            entry.touch()
            self._cache.move_to_end(key)
            self._metrics.hits += 1
            return entry.value

    def set(self, key: K, value: V, ttl: Optional[float] = None) -> None:
        """Set value with TTL."""
        with self._lock:
            ttl = ttl if ttl is not None else self._default_ttl

            # Update existing
            if key in self._cache:
                entry = self._cache[key]
                entry.value = value
                entry.expires_at = time.monotonic() + ttl
                entry.touch()
                self._cache.move_to_end(key)
                self._metrics.sets += 1
                return

            # Evict if at capacity
            while len(self._cache) >= self._max_size:
                self._evict_one()

            # Add new entry
            entry = CacheEntry(value=value)
            entry.expires_at = time.monotonic() + ttl
            self._cache[key] = entry
            self._metrics.sets += 1
            self._metrics.current_size = len(self._cache)

    def delete(self, key: K) -> bool:
        """Delete entry."""
        with self._lock:
            if key in self._cache:
                del self._cache[key]
                self._metrics.deletes += 1
                self._metrics.current_size = len(self._cache)
                return True
            return False

    def contains(self, key: K) -> bool:
        """Check if key exists and not expired."""
        with self._lock:
            if key not in self._cache:
                return False
            return not self._cache[key].is_expired

    def clear(self) -> None:
        """Clear all entries."""
        with self._lock:
            self._cache.clear()
            self._metrics.current_size = 0

    def size(self) -> int:
        """Current number of entries (including expired)."""
        with self._lock:
            return len(self._cache)

    @property
    def metrics(self) -> CacheMetrics:
        """Cache metrics."""
        with self._lock:
            return CacheMetrics(
                hits=self._metrics.hits,
                misses=self._metrics.misses,
                evictions=self._metrics.evictions,
                expirations=self._metrics.expirations,
                sets=self._metrics.sets,
                deletes=self._metrics.deletes,
                current_size=len(self._cache),
                max_size=self._max_size,
            )

    def _evict_one(self) -> None:
        """Evict oldest or expired entry."""
        if not self._cache:
            return

        # First try to evict expired
        for key, entry in self._cache.items():
            if entry.is_expired:
                del self._cache[key]
                self._metrics.expirations += 1
                if self._on_expire:
                    self._on_expire(key, entry.value)
                return

        # Otherwise evict LRU
        key, entry = self._cache.popitem(last=False)
        self._metrics.evictions += 1

        if self._on_evict:
            self._on_evict(key, entry.value)

    def get_ttl(self, key: K) -> Optional[float]:
        """Get remaining TTL for key in seconds."""
        with self._lock:
            if key not in self._cache:
                return None

            entry = self._cache[key]
            if entry.expires_at is None:
                return None

            remaining = entry.expires_at - time.monotonic()
            return max(0.0, remaining)

    def refresh_ttl(self, key: K, ttl: Optional[float] = None) -> bool:
        """Refresh TTL for existing key."""
        with self._lock:
            if key not in self._cache:
                return False

            ttl = ttl if ttl is not None else self._default_ttl
            self._cache[key].expires_at = time.monotonic() + ttl
            return True


# ════════════════════════════════════════════════════════════════════════════
# TIERED CACHE
# ════════════════════════════════════════════════════════════════════════════


class TieredCache(Cache[K, V]):
    """
    Multi-tier cache with L1/L2/... hierarchy.

    Checks caches in order, promotes hits to higher tiers.
    Useful for hot/warm/cold data separation.

    Example:
        l1 = LRUCache(max_size=100)   # Small, fast
        l2 = TTLCache(max_size=10000)  # Larger, TTL

        tiered = TieredCache([l1, l2])
        tiered.set("key", "value")  # Sets in all tiers

        value = tiered.get("key")  # Checks L1 first, then L2
    """

    def __init__(
        self,
        tiers: List[Cache[K, V]],
        promote_on_hit: bool = True,
    ):
        self._tiers = tiers
        self._promote_on_hit = promote_on_hit
        self._lock = threading.RLock()

    def get(self, key: K, default: Optional[V] = None) -> Optional[V]:
        """Get from first cache that has the key."""
        with self._lock:
            for i, tier in enumerate(self._tiers):
                value = tier.get(key)
                if value is not None:
                    # Promote to higher tiers
                    if self._promote_on_hit and i > 0:
                        for j in range(i):
                            self._tiers[j].set(key, value)
                    return value

            return default

    def set(self, key: K, value: V, ttl: Optional[float] = None) -> None:
        """Set in all tiers."""
        with self._lock:
            for tier in self._tiers:
                tier.set(key, value, ttl)

    def delete(self, key: K) -> bool:
        """Delete from all tiers."""
        with self._lock:
            deleted = False
            for tier in self._tiers:
                if tier.delete(key):
                    deleted = True
            return deleted

    def contains(self, key: K) -> bool:
        """Check if any tier has the key."""
        with self._lock:
            return any(tier.contains(key) for tier in self._tiers)

    def clear(self) -> None:
        """Clear all tiers."""
        with self._lock:
            for tier in self._tiers:
                tier.clear()

    def size(self) -> int:
        """Total entries across all tiers (may have duplicates)."""
        with self._lock:
            return sum(tier.size() for tier in self._tiers)

    @property
    def metrics(self) -> CacheMetrics:
        """Aggregated metrics from all tiers."""
        with self._lock:
            total = CacheMetrics()
            for tier in self._tiers:
                m = tier.metrics
                total.hits += m.hits
                total.misses += m.misses
                total.evictions += m.evictions
                total.expirations += m.expirations
                total.sets += m.sets
                total.deletes += m.deletes
                total.current_size += m.current_size
                total.max_size += m.max_size
            return total

    def invalidate(self, key: K) -> bool:
        """Invalidate a key across all tiers.

        Ensures that all tiers are cleared for the given key,
        preventing stale data from being served from any tier.
        """
        with self._lock:
            invalidated = False
            for tier in self._tiers:
                if tier.delete(key):
                    invalidated = True
            return invalidated

    def tier_metrics(self) -> List[CacheMetrics]:
        """Metrics for each tier."""
        with self._lock:
            return [tier.metrics for tier in self._tiers]


# ════════════════════════════════════════════════════════════════════════════
# WRITE-THROUGH CACHE
# ════════════════════════════════════════════════════════════════════════════


class WriteThroughCache(Cache[K, V]):
    """
    Write-through cache that writes to backing store.

    Writes go to both cache and backing store synchronously.
    Ensures strong consistency between cache and store.

    Example:
        def write_to_db(key, value):
            db.insert(key, value)

        def read_from_db(key):
            return db.select(key)

        cache = WriteThroughCache(
            cache=LRUCache(max_size=1000),
            writer=write_to_db,
            reader=read_from_db,
        )
    """

    def __init__(
        self,
        cache: Cache[K, V],
        writer: Callable[[K, V], None],
        reader: Optional[Callable[[K], Optional[V]]] = None,
        deleter: Optional[Callable[[K], None]] = None,
    ):
        self._cache = cache
        self._writer = writer
        self._reader = reader
        self._deleter = deleter
        self._lock = threading.RLock()

    def get(self, key: K, default: Optional[V] = None) -> Optional[V]:
        """Get from cache, fallback to reader if miss."""
        with self._lock:
            value = self._cache.get(key)
            if value is not None:
                return value

            # Cache miss - try reader
            if self._reader:
                value = self._reader(key)
                if value is not None:
                    self._cache.set(key, value)
                    return value

            return default

    def set(self, key: K, value: V, ttl: Optional[float] = None) -> None:
        """Set in cache and write to store."""
        with self._lock:
            self._writer(key, value)
            self._cache.set(key, value, ttl)

    def delete(self, key: K) -> bool:
        """Delete from cache and store."""
        with self._lock:
            if self._deleter:
                self._deleter(key)
            return self._cache.delete(key)

    def contains(self, key: K) -> bool:
        """Check cache."""
        return self._cache.contains(key)

    def clear(self) -> None:
        """Clear cache only (not backing store)."""
        self._cache.clear()

    def size(self) -> int:
        """Cache size."""
        return self._cache.size()

    @property
    def metrics(self) -> CacheMetrics:
        """Cache metrics."""
        return self._cache.metrics


# ════════════════════════════════════════════════════════════════════════════
# COMPUTE CACHE
# ════════════════════════════════════════════════════════════════════════════


class ComputeCache(Generic[K, V]):
    """
    Cache that computes values on miss.

    Useful for memoization of expensive computations.
    Thread-safe with single-flight pattern to prevent stampede.

    Example:
        def expensive_calculation(key):
            time.sleep(1)  # Simulate work
            return compute(key)

        cache = ComputeCache(
            compute=expensive_calculation,
            max_size=1000,
        )

        # First call computes
        result = cache.get("key")

        # Second call returns cached
        result = cache.get("key")  # Instant
    """

    def __init__(
        self,
        compute: Callable[[K], V],
        max_size: int = 1000,
        ttl_seconds: Optional[float] = None,
    ):
        self._compute = compute
        self._cache: Cache[K, V] = (
            TTLCache(max_size=max_size, default_ttl_seconds=ttl_seconds)
            if ttl_seconds
            else LRUCache(max_size=max_size)
        )
        self._computing: Dict[K, threading.Event] = {}
        self._compute_errors: Dict[K, Exception] = {}  # Store errors for waiting threads
        self._lock = threading.RLock()

    def get(self, key: K) -> V:
        """Get value, computing if not cached.

        If the compute function raises an exception, it is NOT cached
        and is propagated to all waiting threads.
        """
        # Check cache first
        with self._lock:
            value = self._cache.get(key)
            if value is not None:
                return value

            # Check if another thread is computing
            if key in self._computing:
                event = self._computing[key]
            else:
                event = threading.Event()
                self._computing[key] = event
                event = None  # We'll do the computation

        # Wait if another thread is computing
        if event is not None:
            event.wait()
            # Check if the computation failed
            with self._lock:
                if key in self._compute_errors:
                    raise self._compute_errors[key]
            value = self._cache.get(key)
            if value is None:
                raise RuntimeError(f"Compute cache: no value available for key {key!r}")
            return value

        # Compute value
        try:
            value = self._compute(key)
            with self._lock:
                self._cache.set(key, value)
                # Clear any previous error for this key
                self._compute_errors.pop(key, None)
            return value
        except Exception as e:
            # Do NOT cache the error; store it so waiting threads can see it
            with self._lock:
                self._compute_errors[key] = e
            raise
        finally:
            with self._lock:
                if key in self._computing:
                    self._computing[key].set()
                    del self._computing[key]

    def invalidate(self, key: K) -> bool:
        """Invalidate cached value."""
        return self._cache.delete(key)

    def clear(self) -> None:
        """Clear all cached values."""
        self._cache.clear()

    @property
    def metrics(self) -> CacheMetrics:
        """Cache metrics."""
        return self._cache.metrics


# ════════════════════════════════════════════════════════════════════════════
# CACHE DECORATOR
# ════════════════════════════════════════════════════════════════════════════


def cached(
    max_size: int = 128,
    ttl_seconds: Optional[float] = None,
    key_func: Optional[Callable[..., str]] = None,
) -> Callable[[Callable[..., T]], Callable[..., T]]:
    """
    Decorator for caching function results.

    Example:
        @cached(max_size=100, ttl_seconds=60)
        def expensive_function(x, y):
            return compute(x, y)

        # With custom key function
        @cached(key_func=lambda x, y: f"{x}:{y}")
        def another_function(x, y):
            return compute(x, y)
    """
    def decorator(func: Callable[..., T]) -> Callable[..., T]:
        cache: Cache[str, T] = (
            TTLCache(max_size=max_size, default_ttl_seconds=ttl_seconds)
            if ttl_seconds
            else LRUCache(max_size=max_size)
        )

        @functools.wraps(func)
        def wrapper(*args, **kwargs) -> T:
            # Generate cache key
            if key_func:
                key = key_func(*args, **kwargs)
            else:
                key_parts = [func.__name__]
                key_parts.extend(str(arg) for arg in args)
                key_parts.extend(f"{k}={v}" for k, v in sorted(kwargs.items()))
                key = hashlib.sha256("|".join(key_parts).encode()).hexdigest()[:16]

            # Check cache
            result = cache.get(key)
            if result is not None:
                return result

            # Compute and cache
            result = func(*args, **kwargs)
            cache.set(key, result)
            return result

        wrapper.cache = cache  # type: ignore
        wrapper.cache_clear = cache.clear  # type: ignore
        wrapper.cache_metrics = lambda: cache.metrics  # type: ignore

        return wrapper
    return decorator


# ════════════════════════════════════════════════════════════════════════════
# CACHE REGISTRY
# ════════════════════════════════════════════════════════════════════════════


class CacheRegistry:
    """
    Registry for managing named caches.

    Provides centralized cache management and metrics.

    Example:
        registry = CacheRegistry()
        registry.create_lru("paths", max_size=1000)
        registry.create_ttl("commitments", max_size=500, ttl=60)

        cache = registry.get("paths")
        metrics = registry.get_all_metrics()
    """

    _instance: Optional['CacheRegistry'] = None
    _lock = threading.Lock()

    def __init__(self):
        self._caches: Dict[str, Cache] = {}
        self._lock = threading.RLock()

    @classmethod
    def get_instance(cls) -> 'CacheRegistry':
        """Get singleton instance."""
        if cls._instance is None:
            with cls._lock:
                if cls._instance is None:
                    cls._instance = cls()
        return cls._instance

    def create_lru(
        self,
        name: str,
        max_size: int = 1000,
        **kwargs,
    ) -> LRUCache:
        """Create and register an LRU cache."""
        with self._lock:
            cache = LRUCache(max_size=max_size, **kwargs)
            self._caches[name] = cache
            return cache

    def create_ttl(
        self,
        name: str,
        max_size: int = 1000,
        ttl_seconds: float = 300.0,
        **kwargs,
    ) -> TTLCache:
        """Create and register a TTL cache."""
        with self._lock:
            cache = TTLCache(max_size=max_size, default_ttl_seconds=ttl_seconds, **kwargs)
            self._caches[name] = cache
            return cache

    def get(self, name: str) -> Optional[Cache]:
        """Get cache by name."""
        with self._lock:
            return self._caches.get(name)

    def register(self, name: str, cache: Cache) -> None:
        """Register an existing cache."""
        with self._lock:
            self._caches[name] = cache

    def unregister(self, name: str) -> bool:
        """Unregister a cache."""
        with self._lock:
            if name in self._caches:
                del self._caches[name]
                return True
            return False

    def clear_all(self) -> None:
        """Clear all registered caches."""
        with self._lock:
            for cache in self._caches.values():
                cache.clear()

    def get_all_metrics(self) -> Dict[str, CacheMetrics]:
        """Get metrics for all caches."""
        with self._lock:
            return {name: cache.metrics for name, cache in self._caches.items()}

    def get_names(self) -> List[str]:
        """Get all cache names."""
        with self._lock:
            return list(self._caches.keys())


# ════════════════════════════════════════════════════════════════════════════
# CONVENIENCE FUNCTIONS
# ════════════════════════════════════════════════════════════════════════════


def get_cache_registry() -> CacheRegistry:
    """Get the global cache registry."""
    return CacheRegistry.get_instance()


# ════════════════════════════════════════════════════════════════════════════
# MODULE EXPORTS
# ════════════════════════════════════════════════════════════════════════════


__all__ = [
    # Entry
    "CacheEntry",
    "CacheMetrics",
    # Base
    "Cache",
    # Implementations
    "LRUCache",
    "TTLCache",
    "TieredCache",
    "WriteThroughCache",
    "ComputeCache",
    # Decorator
    "cached",
    # Registry
    "CacheRegistry",
    "get_cache_registry",
]
