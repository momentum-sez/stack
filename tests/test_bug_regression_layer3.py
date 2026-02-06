"""
Regression Test Suite for PHOENIX Layer 3 Bug Fixes

Tests bug fixes in security.py, watcher.py, and hardening.py to prevent
regressions of previously identified and resolved issues.

Bugs covered:
    Security (security.py):
        #10 - NonceRegistry._cleanup iterates over dict copy (no crash on concurrent modification)
        #11 - RateLimiter uses monotonic time instead of wall-clock time
        #12 - TimeLockManager._cleanup_completed prevents unbounded growth
        #13 - ScopedAttestation.verify_scope checks temporal bounds

    Watcher (watcher.py):
        #15 - Slashing during dict iteration does not crash
        #16 - Reputation score clamped to non-negative
        #17 - Zero bond collateral rejected
        #19 - Division by zero guard in quorum/select with zero watchers

    Hardening (hardening.py):
        #63 - ThreadSafeDict.__iter__ yields from a snapshot
        #64 - CryptoUtils.merkle_root does not mutate the input list
"""

import hashlib
import secrets
import threading
import time
import unittest
from datetime import datetime, timedelta, timezone
from decimal import Decimal
from unittest.mock import patch

from tools.phoenix.hardening import (
    CryptoUtils,
    RateLimitConfig,
    RateLimiter,
    ThreadSafeDict,
)
from tools.phoenix.security import (
    AttestationScope,
    NonceRegistry,
    ScopedAttestation,
    TimeLock,
    TimeLockManager,
    TimeLockState,
)
from tools.phoenix.watcher import (
    BondStatus,
    ReputationMetrics,
    SlashingClaim,
    SlashingCondition,
    SlashingEvidence,
    WatcherBond,
    WatcherId,
    WatcherRegistry,
    WatcherReputation,
)


# =============================================================================
# HELPERS
# =============================================================================


def _make_watcher_id(name: str = "alice") -> WatcherId:
    """Create a deterministic WatcherId for tests."""
    return WatcherId(
        did=f"did:example:{name}",
        public_key_hex="ab" * 32,
    )


def _make_bond(
    watcher_id: WatcherId,
    bond_id: str = "bond-001",
    collateral: Decimal = Decimal("10000"),
) -> WatcherBond:
    """Create a WatcherBond suitable for testing."""
    return WatcherBond(
        bond_id=bond_id,
        watcher_id=watcher_id,
        collateral_amount=collateral,
        collateral_currency="USDC",
        collateral_address="0x" + "aa" * 20,
        scope_jurisdictions=frozenset({"us-de", "sg-main"}),
        scope_asset_classes=frozenset({"equity", "bond"}),
    )


def _register_and_activate(
    registry: WatcherRegistry,
    watcher_id: WatcherId,
    bond: WatcherBond,
) -> None:
    """Register a watcher, post and activate their bond."""
    registry.register_watcher(watcher_id)
    registry.post_bond(bond)
    registry.activate_bond(bond.bond_id)


# =============================================================================
# SECURITY.PY REGRESSION TESTS
# =============================================================================


class TestNonceManagerConcurrency(unittest.TestCase):
    """
    Bug #10: NonceRegistry._cleanup iterates over dict copy.

    Previously _cleanup iterated over self._nonces.items() directly, which
    could raise RuntimeError if another thread mutated the dict during
    cleanup.  The fix uses list(self._nonces.items()) to snapshot.
    """

    def test_cleanup_does_not_crash_on_concurrent_modification(self):
        """Concurrent register + cleanup must never raise RuntimeError."""
        registry = NonceRegistry(max_age_hours=0)  # everything expires instantly

        # Pre-populate so there is something to clean up
        for i in range(200):
            registry.check_and_register(f"seed-nonce-{i}")

        errors = []
        stop = threading.Event()

        def registerer():
            """Continuously register new nonces."""
            idx = 0
            while not stop.is_set():
                try:
                    registry.check_and_register(f"reg-{threading.current_thread().name}-{idx}")
                    idx += 1
                except RuntimeError as exc:
                    errors.append(exc)
                    break

        threads = [threading.Thread(target=registerer, name=f"t-{i}") for i in range(8)]
        for t in threads:
            t.start()

        # Let them race for a while
        time.sleep(0.3)
        stop.set()
        for t in threads:
            t.join(timeout=5)

        self.assertEqual(errors, [], "Concurrent nonce operations caused RuntimeError")

    def test_cleanup_removes_expired_nonces(self):
        """Expired nonces should be removed during cleanup."""
        registry = NonceRegistry(max_age_hours=0)  # instant expiry

        registry.check_and_register("old-nonce")
        # Give a tiny window for the nonce to be "old"
        time.sleep(0.01)

        # The next call triggers _cleanup internally
        registry.check_and_register("new-nonce")

        # old-nonce should have been cleaned up, so it appears fresh again
        self.assertTrue(
            registry.is_fresh("old-nonce"),
            "Expired nonce was not cleaned up",
        )

    def test_replay_detection(self):
        """Registering the same nonce twice must return False on the second call."""
        registry = NonceRegistry()
        self.assertTrue(registry.check_and_register("unique-nonce"))
        self.assertFalse(registry.check_and_register("unique-nonce"))


class TestRateLimiterMonotonicTime(unittest.TestCase):
    """
    Bug #11: RateLimiter must use time.monotonic(), not time.time().

    Wall-clock time (time.time()) can jump backwards due to NTP adjustments,
    DST changes, or manual clock changes, causing the rate limiter to
    incorrectly compute elapsed time and either over-grant or starve tokens.
    """

    def test_rate_limiter_uses_monotonic_time(self):
        """Verify time.monotonic is used, not time.time."""
        config = RateLimitConfig(requests_per_minute=600, burst_size=5)
        limiter = RateLimiter(config)  # 10 tokens/sec refill rate

        # Exhaust burst
        for _ in range(5):
            self.assertTrue(limiter.acquire())
        self.assertFalse(limiter.acquire(), "Should be rate limited after burst")

        # Simulate wall-clock going backwards by 1 hour - should NOT affect
        # the limiter because it uses monotonic time.
        with patch("time.time", return_value=time.time() - 3600):
            # Wait for monotonic refill (10 tokens/sec, so 0.2s ~ 2 tokens)
            time.sleep(0.25)
            # Monotonic clock advanced, so we should get a token
            self.assertTrue(
                limiter.acquire(),
                "Limiter should refill tokens based on monotonic time, not wall clock",
            )

    def test_rate_limiter_refills_tokens_over_time(self):
        """Tokens should refill at the configured rate."""
        config = RateLimitConfig(requests_per_minute=600, burst_size=5)
        limiter = RateLimiter(config)

        # Exhaust burst
        for _ in range(5):
            limiter.acquire()

        # At 600/min = 10/sec, 0.2s should refill ~2 tokens
        time.sleep(0.25)
        self.assertTrue(limiter.acquire(), "Token should have refilled")

    def test_rate_limiter_respects_burst_cap(self):
        """Tokens must never exceed the burst_size ceiling."""
        config = RateLimitConfig(requests_per_minute=6000, burst_size=3)
        limiter = RateLimiter(config)

        # Wait a moment for potential over-refill
        time.sleep(0.2)

        # Should only be able to acquire burst_size tokens
        acquired = sum(1 for _ in range(10) if limiter.acquire())
        self.assertLessEqual(acquired, config.burst_size + 1,
                             "Acquired more tokens than burst cap allows")


class TestTimeLockCleanup(unittest.TestCase):
    """
    Bug #12: TimeLockManager._cleanup_completed prevents unbounded growth.

    Without cleanup, executed/cancelled/expired locks accumulated indefinitely,
    causing memory growth proportional to total operations over the system
    lifetime.  The fix calls _cleanup_completed after each execution.
    """

    def test_timelock_cleans_up_expired_operations(self):
        """Executed locks should be removed to prevent unbounded growth."""
        manager = TimeLockManager()

        # Create a lock that is immediately unlockable
        now = datetime.now(timezone.utc)
        past = now - timedelta(hours=2)
        future = now + timedelta(hours=2)

        operation_data = b"test-operation-data"
        commitment = hashlib.sha256(operation_data).hexdigest()

        lock = TimeLock(
            lock_id="tl-test-cleanup",
            operation_type="migration",
            operator_did="did:example:operator",
            announced_at=past.isoformat(),
            unlock_at=past.isoformat(),
            expires_at=future.isoformat(),
            operation_commitment=commitment,
        )

        # Inject lock directly to bypass time delays
        manager._locks["tl-test-cleanup"] = lock

        # Execute the lock
        success, msg = manager.execute("tl-test-cleanup", operation_data)
        self.assertTrue(success, f"Execution should succeed: {msg}")

        # After execution, the completed lock should be cleaned up
        self.assertNotIn(
            "tl-test-cleanup",
            manager._locks,
            "Executed lock should be cleaned up to prevent unbounded growth",
        )

    def test_cancelled_locks_cleaned_up(self):
        """Cancelled locks should also be cleaned up during the next execute."""
        manager = TimeLockManager()

        now = datetime.now(timezone.utc)
        past = now - timedelta(hours=2)
        future = now + timedelta(hours=2)

        # Create and cancel a lock
        cancelled_lock = TimeLock(
            lock_id="tl-cancelled",
            operation_type="migration",
            operator_did="did:example:operator",
            announced_at=past.isoformat(),
            unlock_at=past.isoformat(),
            expires_at=future.isoformat(),
            operation_commitment="dummy",
            state=TimeLockState.CANCELLED,
        )
        manager._locks["tl-cancelled"] = cancelled_lock

        # Create and execute another lock to trigger cleanup
        operation_data = b"trigger-cleanup"
        commitment = hashlib.sha256(operation_data).hexdigest()
        active_lock = TimeLock(
            lock_id="tl-active",
            operation_type="migration",
            operator_did="did:example:operator",
            announced_at=past.isoformat(),
            unlock_at=past.isoformat(),
            expires_at=future.isoformat(),
            operation_commitment=commitment,
        )
        manager._locks["tl-active"] = active_lock

        manager.execute("tl-active", operation_data)

        self.assertNotIn(
            "tl-cancelled",
            manager._locks,
            "Cancelled lock should be cleaned up during the next execute",
        )

    def test_pending_locks_not_prematurely_cleaned(self):
        """Valid pending locks must not be removed by cleanup."""
        manager = TimeLockManager()

        lock = manager.announce(
            operation_type="migration",
            operator_did="did:example:operator",
            operation_commitment="abc123",
            delay_hours=24,
        )

        # Pending lock should survive
        pending = manager.get_pending_locks()
        self.assertTrue(
            any(pl.lock_id == lock.lock_id for pl in pending),
            "Valid pending lock should not be cleaned up",
        )


class TestScopedAttestationTemporalValidity(unittest.TestCase):
    """
    Bug #13: ScopedAttestation.verify_scope must check temporal bounds.

    Without the temporal check, an attestation could be accepted even after
    its validity period expired, or before it becomes active.
    """

    def _make_scope(self, valid_from: datetime, valid_until: datetime) -> AttestationScope:
        return AttestationScope(
            asset_id="asset-001",
            jurisdiction_id="us-de",
            domain="transfer",
            valid_from=valid_from.isoformat(),
            valid_until=valid_until.isoformat(),
        )

    def test_scoped_attestation_checks_temporal_validity(self):
        """Attestation outside its valid window should fail verification."""
        now = datetime.now(timezone.utc)
        past_scope = self._make_scope(
            valid_from=now - timedelta(days=30),
            valid_until=now - timedelta(days=1),
        )

        attestation = ScopedAttestation.create(
            attestation_id="att-expired",
            attestation_type="transfer",
            issuer_did="did:example:issuer",
            scope=past_scope,
            issuer_signature=b"\x00" * 64,
        )

        # Should fail because scope has expired
        result = attestation.verify_scope(
            asset_id="asset-001",
            jurisdiction_id="us-de",
            domain="transfer",
            at_time=now,
        )
        self.assertFalse(result, "Expired attestation should fail temporal check")

    def test_scoped_attestation_rejects_future_scope(self):
        """Attestation not yet valid should fail verification."""
        now = datetime.now(timezone.utc)
        future_scope = self._make_scope(
            valid_from=now + timedelta(days=10),
            valid_until=now + timedelta(days=30),
        )

        attestation = ScopedAttestation.create(
            attestation_id="att-future",
            attestation_type="transfer",
            issuer_did="did:example:issuer",
            scope=future_scope,
            issuer_signature=b"\x00" * 64,
        )

        result = attestation.verify_scope(
            asset_id="asset-001",
            jurisdiction_id="us-de",
            domain="transfer",
            at_time=now,
        )
        self.assertFalse(result, "Not-yet-valid attestation should fail temporal check")

    def test_scoped_attestation_accepts_valid_window(self):
        """Attestation within valid window should pass verification."""
        now = datetime.now(timezone.utc)
        valid_scope = self._make_scope(
            valid_from=now - timedelta(days=1),
            valid_until=now + timedelta(days=1),
        )

        attestation = ScopedAttestation.create(
            attestation_id="att-valid",
            attestation_type="transfer",
            issuer_did="did:example:issuer",
            scope=valid_scope,
            issuer_signature=b"\x00" * 64,
        )

        result = attestation.verify_scope(
            asset_id="asset-001",
            jurisdiction_id="us-de",
            domain="transfer",
            at_time=now,
        )
        self.assertTrue(result, "Attestation in valid window should pass")

    def test_scoped_attestation_rejects_wrong_context(self):
        """Attestation checked against wrong context should fail."""
        now = datetime.now(timezone.utc)
        scope = self._make_scope(
            valid_from=now - timedelta(days=1),
            valid_until=now + timedelta(days=1),
        )

        attestation = ScopedAttestation.create(
            attestation_id="att-wrong-ctx",
            attestation_type="transfer",
            issuer_did="did:example:issuer",
            scope=scope,
            issuer_signature=b"\x00" * 64,
        )

        # Wrong asset_id
        self.assertFalse(
            attestation.verify_scope("wrong-asset", "us-de", "transfer", at_time=now),
            "Wrong asset_id should fail scope check",
        )
        # Wrong jurisdiction
        self.assertFalse(
            attestation.verify_scope("asset-001", "sg-main", "transfer", at_time=now),
            "Wrong jurisdiction should fail scope check",
        )
        # Wrong domain
        self.assertFalse(
            attestation.verify_scope("asset-001", "us-de", "custody", at_time=now),
            "Wrong domain should fail scope check",
        )


# =============================================================================
# WATCHER.PY REGRESSION TESTS
# =============================================================================


class TestWatcherEconomics(unittest.TestCase):
    """Regression tests for watcher economic invariants."""

    def test_reputation_score_clamped_to_non_negative(self):
        """
        Bug #16: Reputation score must never go below 0.

        A watcher with many slash incidents could produce a negative raw
        score from the penalty calculation.  The fix clamps with max(0.0, ...).
        """
        watcher_id = _make_watcher_id("slashed-alice")
        metrics = ReputationMetrics(
            required_attestations=100,
            delivered_attestations=10,  # very low availability
            on_time_attestations=5,
            challenged_attestations=20,
            successful_challenges=20,  # 100% challenge success = 0% accuracy
            failed_challenges=0,
            slash_incidents=10,  # massive penalty: 10 * 10 = 100 points
            total_slashed_usd=Decimal("50000"),
            continuous_active_days=0,
        )

        reputation = WatcherReputation(
            watcher_id=watcher_id,
            metrics=metrics,
        )

        score = reputation.compute_score()
        self.assertGreaterEqual(
            score, 0.0,
            f"Reputation score should never be negative, got {score}",
        )
        self.assertLessEqual(
            score, 100.0,
            f"Reputation score should not exceed 100, got {score}",
        )

    def test_reputation_score_clamped_to_max_100(self):
        """Perfect metrics should cap at 100.0, not exceed it."""
        watcher_id = _make_watcher_id("perfect-bob")
        metrics = ReputationMetrics(
            required_attestations=1000,
            delivered_attestations=1000,
            on_time_attestations=1000,
            challenged_attestations=50,
            successful_challenges=0,
            failed_challenges=50,
            slash_incidents=0,
            total_slashed_usd=Decimal("0"),
            continuous_active_days=400,
        )

        reputation = WatcherReputation(
            watcher_id=watcher_id,
            metrics=metrics,
        )
        score = reputation.compute_score()
        self.assertLessEqual(score, 100.0, "Score must not exceed 100")
        self.assertGreaterEqual(score, 0.0, "Score must not be negative")

    def test_zero_bond_collateral_rejected(self):
        """
        Bug #17: WatcherRegistry.post_bond must reject zero collateral bonds.

        Posting a bond with zero (or negative) collateral would allow a watcher
        to operate with no economic stake, defeating the accountability model.
        """
        registry = WatcherRegistry()
        watcher_id = _make_watcher_id("cheapskate")
        registry.register_watcher(watcher_id)

        zero_bond = _make_bond(watcher_id, bond_id="bond-zero", collateral=Decimal("0"))
        result = registry.post_bond(zero_bond)
        self.assertFalse(result, "Zero collateral bond should be rejected")

        negative_bond = _make_bond(watcher_id, bond_id="bond-neg", collateral=Decimal("-100"))
        result = registry.post_bond(negative_bond)
        self.assertFalse(result, "Negative collateral bond should be rejected")

    def test_valid_bond_accepted(self):
        """A bond with positive collateral should be accepted."""
        registry = WatcherRegistry()
        watcher_id = _make_watcher_id("honest")
        registry.register_watcher(watcher_id)

        bond = _make_bond(watcher_id, bond_id="bond-ok", collateral=Decimal("5000"))
        result = registry.post_bond(bond)
        self.assertTrue(result, "Valid bond should be accepted")

    def test_quorum_with_zero_watchers(self):
        """
        Bug #19: select_watchers must handle empty registry without division by zero.

        When no watchers are registered, any code that computes quorum ratios
        (e.g. selected_count / total_watchers) would crash with ZeroDivisionError.
        The fix adds an early return for empty registries.
        """
        registry = WatcherRegistry()

        # Must not raise ZeroDivisionError or any other exception
        result = registry.select_watchers(jurisdiction_id="us-de", min_count=3)
        self.assertEqual(result, [], "Empty registry should return empty list")

    def test_quorum_with_no_active_bonds(self):
        """select_watchers should return empty list when no bonds are active."""
        registry = WatcherRegistry()
        watcher_id = _make_watcher_id("no-bond")
        registry.register_watcher(watcher_id)

        result = registry.select_watchers(jurisdiction_id="us-de", min_count=1)
        self.assertEqual(result, [], "No active bonds should yield empty selection")

    def test_slashing_during_iteration_doesnt_crash(self):
        """
        Bug #15: Dict iteration safety during slashing and registry export.

        When a watcher is slashed and banned (e.g., for collusion) while another
        thread is iterating over the watcher registry (export_registry,
        select_watchers), iterating over list(dict.items()) prevents
        RuntimeError from dictionary size change during iteration.
        """
        registry = WatcherRegistry()

        # Register multiple watchers
        watchers = []
        for i in range(20):
            wid = _make_watcher_id(f"watcher-{i}")
            bond = _make_bond(wid, bond_id=f"bond-{i}", collateral=Decimal("10000"))
            _register_and_activate(registry, wid, bond)
            watchers.append(wid)

        errors = []
        stop = threading.Event()

        def iterate_registry():
            """Continuously call export_registry which iterates all dicts."""
            while not stop.is_set():
                try:
                    registry.export_registry()
                    registry.select_watchers("us-de", min_count=5)
                    registry.get_statistics()
                except RuntimeError as exc:
                    errors.append(("iteration", exc))
                    break

        def slash_watchers():
            """File and execute slashing claims."""
            for i, wid in enumerate(watchers):
                if stop.is_set():
                    break
                evidence = SlashingEvidence(
                    evidence_type="test",
                    evidence_data={"reason": f"test-slash-{i}"},
                )
                claim = SlashingClaim(
                    claim_id=f"claim-{i}",
                    watcher_id=wid,
                    condition=SlashingCondition.EQUIVOCATION,
                    evidence=evidence,
                    claimant_did="did:example:prosecutor",
                    challenge_deadline=(
                        datetime.now(timezone.utc) - timedelta(days=1)
                    ).isoformat(),
                )
                try:
                    registry.file_slashing_claim(claim)
                    registry.execute_claim(claim.claim_id)
                except RuntimeError as exc:
                    errors.append(("slash", exc))
                    break

        iter_threads = [
            threading.Thread(target=iterate_registry, name=f"iter-{i}")
            for i in range(4)
        ]
        slash_thread = threading.Thread(target=slash_watchers, name="slasher")

        for t in iter_threads:
            t.start()
        slash_thread.start()

        slash_thread.join(timeout=10)
        stop.set()
        for t in iter_threads:
            t.join(timeout=5)

        self.assertEqual(
            errors, [],
            f"Dict iteration during concurrent slashing caused errors: {errors}",
        )


class TestWatcherSlashingDetails(unittest.TestCase):
    """Additional slashing invariant tests."""

    def test_slash_does_not_go_below_zero(self):
        """Slashing more than available collateral should not produce negative balance."""
        watcher_id = _make_watcher_id("over-slashed")
        bond = _make_bond(watcher_id, collateral=Decimal("1000"))
        bond.status = BondStatus.ACTIVE

        # Slash more than available
        actual = bond.slash(Decimal("5000"), "test")
        self.assertEqual(actual, Decimal("1000"), "Slash should be capped to available")
        self.assertEqual(bond.available_collateral, Decimal("0"))
        self.assertEqual(bond.status, BondStatus.FULLY_SLASHED)

    def test_zero_collateral_bond_execute_claim(self):
        """Executing a claim on a zero-collateral bond should not crash."""
        registry = WatcherRegistry()
        watcher_id = _make_watcher_id("zero-col")
        bond = _make_bond(watcher_id, bond_id="bond-zc", collateral=Decimal("1000"))
        _register_and_activate(registry, watcher_id, bond)

        # Slash to zero first
        bond.slash(Decimal("1000"), "drain")

        evidence = SlashingEvidence(
            evidence_type="test",
            evidence_data={"reason": "test"},
        )
        claim = SlashingClaim(
            claim_id="claim-zc",
            watcher_id=watcher_id,
            condition=SlashingCondition.FALSE_ATTESTATION,
            evidence=evidence,
            claimant_did="did:example:claimant",
            challenge_deadline=(
                datetime.now(timezone.utc) - timedelta(days=1)
            ).isoformat(),
        )
        registry.file_slashing_claim(claim)

        # Should not crash even though bond is fully slashed
        result = registry.execute_claim("claim-zc")
        self.assertIsNotNone(result, "execute_claim should return a value, not crash")


# =============================================================================
# HARDENING.PY REGRESSION TESTS
# =============================================================================


class TestThreadSafeDictIteration(unittest.TestCase):
    """
    Bug #63: ThreadSafeDict.__iter__ should yield from a snapshot.

    Without snapshotting, iterating over a ThreadSafeDict while another thread
    modifies it would raise RuntimeError (dictionary changed size during
    iteration).  The fix captures list(super().keys()) under the lock and
    iterates over the snapshot.
    """

    def test_iteration_returns_snapshot(self):
        """Modifying a ThreadSafeDict during iteration should not crash."""
        d = ThreadSafeDict()
        for i in range(100):
            d[f"key-{i}"] = i

        # Iterate and mutate concurrently (same thread)
        collected = []
        for key in d:
            collected.append(key)
            # Add a new key during iteration -- this should not crash
            d[f"new-{key}"] = "added"

        self.assertGreater(
            len(collected), 0,
            "Iteration should have yielded some keys",
        )
        # The snapshot should reflect the state at iteration start,
        # so new keys added during iteration should NOT appear
        for key in collected:
            self.assertTrue(
                key.startswith("key-"),
                f"Unexpected key from iteration: {key}",
            )

    def test_concurrent_modification_during_iteration_safe(self):
        """Multiple threads modifying dict while others iterate must not crash."""
        d = ThreadSafeDict()
        for i in range(200):
            d[f"init-{i}"] = i

        errors = []
        stop = threading.Event()

        def writer():
            idx = 0
            while not stop.is_set():
                try:
                    d[f"w-{threading.current_thread().name}-{idx}"] = idx
                    idx += 1
                    # Also do some deletions
                    if idx % 3 == 0:
                        key_to_del = f"w-{threading.current_thread().name}-{idx - 3}"
                        d.pop(key_to_del, None)
                except RuntimeError as exc:
                    errors.append(("writer", exc))
                    break

        def reader():
            while not stop.is_set():
                try:
                    keys = list(d)  # triggers __iter__
                    _ = len(keys)
                except RuntimeError as exc:
                    errors.append(("reader", exc))
                    break

        writers = [threading.Thread(target=writer, name=f"w-{i}") for i in range(4)]
        readers = [threading.Thread(target=reader, name=f"r-{i}") for i in range(4)]

        for t in writers + readers:
            t.start()

        time.sleep(0.4)
        stop.set()

        for t in writers + readers:
            t.join(timeout=5)

        self.assertEqual(
            errors, [],
            f"Concurrent dict iteration/modification caused errors: {errors}",
        )

    def test_iter_under_transaction(self):
        """Iteration inside a transaction context should also be safe."""
        d = ThreadSafeDict()
        d["a"] = 1
        d["b"] = 2
        d["c"] = 3

        with d.transaction():
            keys = list(d)
            d["d"] = 4  # modify during transaction

        self.assertIn("a", keys)
        self.assertIn("b", keys)
        self.assertIn("c", keys)
        # "d" was added after the iteration snapshot
        self.assertIn("d", d)

    def test_len_is_thread_safe(self):
        """__len__ should return consistent results under contention."""
        d = ThreadSafeDict()
        errors = []

        def add_keys():
            for i in range(100):
                d[f"k-{threading.current_thread().name}-{i}"] = i

        def check_len():
            try:
                for _ in range(50):
                    n = len(d)
                    self.assertGreaterEqual(n, 0)
            except Exception as exc:
                errors.append(exc)

        threads = []
        for i in range(4):
            threads.append(threading.Thread(target=add_keys))
            threads.append(threading.Thread(target=check_len))

        for t in threads:
            t.start()
        for t in threads:
            t.join(timeout=5)

        self.assertEqual(errors, [])


class TestCryptoUtilsMerkleRoot(unittest.TestCase):
    """
    Bug #64: CryptoUtils.merkle_root must not mutate the input list.

    The original implementation appended padding elements directly to the
    input list when the leaf count was odd.  The fix copies the input with
    list(leaves) before processing.
    """

    def test_merkle_root_does_not_mutate_input(self):
        """Input leaf list must be unchanged after merkle_root computation."""
        original = ["a" * 64, "b" * 64, "c" * 64]
        copy = list(original)

        CryptoUtils.merkle_root(original)

        self.assertEqual(
            original, copy,
            "merkle_root must not mutate the input list "
            f"(original now has {len(original)} items, expected {len(copy)})",
        )

    def test_merkle_root_does_not_mutate_even_length_input(self):
        """Even-length inputs should also remain unchanged."""
        original = ["a" * 64, "b" * 64, "c" * 64, "d" * 64]
        copy = list(original)

        CryptoUtils.merkle_root(original)

        self.assertEqual(
            original, copy,
            "merkle_root must not mutate even-length input",
        )

    def test_merkle_root_does_not_mutate_single_leaf(self):
        """Single-leaf input should remain unchanged."""
        original = ["a" * 64]
        copy = list(original)

        result = CryptoUtils.merkle_root(original)

        self.assertEqual(original, copy)
        self.assertEqual(result, "a" * 64, "Single leaf should be returned as root")

    def test_merkle_root_empty_input(self):
        """Empty list should return zero-hash without error."""
        original = []
        result = CryptoUtils.merkle_root(original)
        self.assertEqual(result, "0" * 64)
        self.assertEqual(original, [], "Empty list should remain empty")

    def test_merkle_root_deterministic(self):
        """Same inputs must always produce the same root."""
        leaves = [hashlib.sha256(f"leaf-{i}".encode()).hexdigest() for i in range(7)]

        root1 = CryptoUtils.merkle_root(list(leaves))
        root2 = CryptoUtils.merkle_root(list(leaves))

        self.assertEqual(root1, root2, "Merkle root should be deterministic")

    def test_merkle_root_different_inputs_different_roots(self):
        """Different inputs must produce different roots."""
        leaves_a = ["a" * 64, "b" * 64]
        leaves_b = ["c" * 64, "d" * 64]

        root_a = CryptoUtils.merkle_root(leaves_a)
        root_b = CryptoUtils.merkle_root(leaves_b)

        self.assertNotEqual(root_a, root_b, "Different leaves should produce different roots")

    def test_merkle_root_repeated_calls_same_list(self):
        """Calling merkle_root multiple times on the same list should always work."""
        leaves = ["a" * 64, "b" * 64, "c" * 64]
        results = set()

        for _ in range(10):
            results.add(CryptoUtils.merkle_root(leaves))

        self.assertEqual(len(results), 1, "Repeated calls should be deterministic")
        self.assertEqual(
            leaves, ["a" * 64, "b" * 64, "c" * 64],
            "Original list must be intact after repeated calls",
        )


# =============================================================================
# CROSS-CUTTING INTEGRATION TESTS
# =============================================================================


class TestSecurityWatcherIntegration(unittest.TestCase):
    """Integration tests combining security and watcher components."""

    def test_full_watcher_lifecycle_with_reputation(self):
        """Register, bond, attest, slash, and verify reputation stays bounded."""
        registry = WatcherRegistry()
        watcher_id = _make_watcher_id("lifecycle")
        bond = _make_bond(watcher_id, collateral=Decimal("10000"))
        _register_and_activate(registry, watcher_id, bond)

        # Record successful attestations
        for _ in range(50):
            registry.record_attestation(watcher_id.did, Decimal("100"), on_time=True)

        rep = registry.get_reputation(watcher_id.did)
        self.assertIsNotNone(rep)
        score_before = rep.compute_score()
        self.assertGreater(score_before, 0)

        # Slash heavily
        for i in range(5):
            evidence = SlashingEvidence(
                evidence_type="test",
                evidence_data={"iteration": i},
            )
            claim = SlashingClaim(
                claim_id=f"lifecycle-claim-{i}",
                watcher_id=watcher_id,
                condition=SlashingCondition.AVAILABILITY_FAILURE,
                evidence=evidence,
                claimant_did="did:example:prosecutor",
                challenge_deadline=(
                    datetime.now(timezone.utc) - timedelta(days=1)
                ).isoformat(),
            )
            registry.file_slashing_claim(claim)
            registry.execute_claim(claim.claim_id)

        score_after = rep.compute_score()
        self.assertGreaterEqual(score_after, 0.0, "Score must remain non-negative")
        self.assertLessEqual(score_after, 100.0, "Score must remain within bounds")

    def test_nonce_registry_with_attestation_flow(self):
        """NonceRegistry correctly prevents attestation replay."""
        nonce_reg = NonceRegistry()

        nonce = secrets.token_hex(16)
        self.assertTrue(nonce_reg.check_and_register(nonce))
        self.assertFalse(
            nonce_reg.check_and_register(nonce),
            "Replayed nonce should be rejected",
        )
        self.assertEqual(nonce_reg.size(), 1)


if __name__ == "__main__":
    unittest.main()
