"""
Tests for migration deadline enforcement (TIER 2C).

Verifies that:
1. Expired deadlines raise MigrationTimeoutError on advance.
2. Future deadlines allow normal advance.
3. Terminal state migrations don't raise even if deadline passed.
"""
from datetime import datetime, timedelta, timezone

import pytest

from tools.phoenix.migration import (
    MigrationRequest,
    MigrationSaga,
    MigrationState,
    MigrationTimeoutError,
)


def _make_request() -> MigrationRequest:
    """Create a minimal migration request for testing."""
    return MigrationRequest(
        asset_id="test-asset-001",
        asset_genesis_digest="a" * 64,
        source_jurisdiction="PK",
        target_jurisdiction="AE",
        requestor_did="did:key:z6MkTest",
    )


def test_expired_deadline_raises_timeout():
    """A migration with an expired deadline must raise on advance."""
    past_deadline = datetime.now(timezone.utc) - timedelta(hours=1)
    saga = MigrationSaga(
        request=_make_request(),
        deadline=past_deadline,
    )
    assert saga.state == MigrationState.INITIATED
    with pytest.raises(MigrationTimeoutError, match="exceeded deadline"):
        saga.advance_to(MigrationState.COMPLIANCE_CHECK, reason="test")


def test_future_deadline_allows_advance():
    """A migration with a future deadline should advance normally."""
    future_deadline = datetime.now(timezone.utc) + timedelta(hours=24)
    saga = MigrationSaga(
        request=_make_request(),
        deadline=future_deadline,
    )
    result = saga.advance_to(MigrationState.COMPLIANCE_CHECK, reason="test")
    assert result is True
    assert saga.state == MigrationState.COMPLIANCE_CHECK


def test_no_deadline_allows_advance():
    """A migration with no deadline set should advance normally."""
    saga = MigrationSaga(request=_make_request())
    result = saga.advance_to(MigrationState.COMPLIANCE_CHECK, reason="test")
    assert result is True
    assert saga.state == MigrationState.COMPLIANCE_CHECK


def test_terminal_state_no_timeout_raise():
    """A migration already in terminal state should not raise on deadline check."""
    past_deadline = datetime.now(timezone.utc) - timedelta(hours=1)
    saga = MigrationSaga(
        request=_make_request(),
        deadline=past_deadline,
    )
    # Force to terminal state
    saga._state = MigrationState.COMPLETED
    # _check_deadline should not raise for terminal states
    saga._check_deadline()  # Should not raise


def test_expired_deadline_triggers_compensation():
    """Expired deadline should trigger compensation and record it."""
    past_deadline = datetime.now(timezone.utc) - timedelta(hours=1)
    saga = MigrationSaga(
        request=_make_request(),
        deadline=past_deadline,
    )
    with pytest.raises(MigrationTimeoutError):
        saga.advance_to(MigrationState.COMPLIANCE_CHECK, reason="test")
    # After timeout, state should be COMPENSATED
    assert saga.state == MigrationState.COMPENSATED
